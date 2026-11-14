-- The GNet interval: every minute, sync server and player information to GravNet.
-- POST /gnet/server/{SERVER_ID}/heartbeat (see GravNet.HTTP.ServerRequest).
-- The first successful sync after linking is the server's first heartbeat: the backend fills in
-- gamemode and map from it (docs/SERVERS.md).

GravNet.Interval = GravNet.Interval or {}

local TIMER = "GravNet.Interval"

-- Player sessions by SteamID64. Kept across failed syncs so no playtime is lost while GravNet
-- is unreachable; a disconnected player's session is dropped once it has been reported.
--   { name, usergroup, connected_at (unix seconds), counted_at, pending (unreported seconds), online,
--     stats (latest gamemode stat values), counters (unreported gamemode counters) }
-- Gamemode stats come from sv_stats.lua.
GravNet.Interval.Sessions = GravNet.Interval.Sessions or {}
local Sessions = GravNet.Interval.Sessions

-- Sessions kept across a Lua refresh may predate the stat fields.
for _, session in pairs(Sessions) do
	session.stats = session.stats or {}
	session.counters = session.counters or {}
end

-- Most a counter can grow by in one heartbeat; the backend caps it there too. The rest waits for
-- the next heartbeat.
local MAX_COUNTER = 1000000

-- The backend refuses a heartbeat with more than 256 players or a body over 256 KiB. Sessions
-- pile up while GravNet is unreachable (disconnected players are kept until reported), so a
-- heartbeat reports at most this much, online players first, and the rest wait for the next
-- ones. Otherwise one oversized heartbeat would be refused every minute forever.
local MAX_PLAYERS = 256
local MAX_BODY_BYTES = 200 * 1024

-- True while a heartbeat is in flight. Two at once would report the same playtime twice.
local InFlight = false

-- Set when linking; the next successful heartbeat is announced as the first one.
local AwaitingFirst = false

-- The latest sync attempt: { time = os.time(), ok = bool, message = string } or nil.
local LastSync = nil

function GravNet.Interval.IsBusy()
	return InFlight
end

function GravNet.Interval.GetLastSync()
	return LastSync
end

-- Start (or resume) tracking a player's session. Returns the session, or nil for bots.
local function Track(ply)
	if not IsValid(ply) or ply:IsBot() then return end
	local steam_id = ply:SteamID64()
	if not steam_id or steam_id == "0" then return end

	local now = os.time()
	local session = Sessions[steam_id]
	if not session or not session.online then
		session = session or { pending = 0, stats = {}, counters = {} }
		session.connected_at = now
		session.counted_at = now
		session.online = true
		Sessions[steam_id] = session
	end

	session.name = ply:Nick()
	session.usergroup = ply:GetUserGroup()
	return session
end

-- Refresh a session's gamemode stat values. A value the gamemode doesn't have right now (e.g.
-- DarkRP money before it has loaded) keeps its last known value.
local function SampleStats(ply, session)
	for key, value in pairs(GravNet.Stats.Sample(ply)) do
		session.stats[key] = value
	end
end

-- Add to one of a player's gamemode counters (see GravNet.Stats.Count).
function GravNet.Interval.Count(ply, key, amount)
	if not isnumber(amount) or amount <= 0 then return end
	local session = Track(ply)
	if session then session.counters[key] = (session.counters[key] or 0) + amount end
end

-- Move playtime since the last count into `pending`.
local function Accrue(session, now)
	session.pending = session.pending + math.max(0, now - session.counted_at)
	session.counted_at = now
end

hook.Add("PlayerInitialSpawn", "GravNet.Interval.Track", function(ply)
	Track(ply)
end)

hook.Add("PlayerDisconnected", "GravNet.Interval.Track", function(ply)
	if ply:IsBot() then return end
	local session = Sessions[ply:SteamID64()]
	if session and session.online then
		Accrue(session, os.time())
		SampleStats(ply, session)
		session.online = false
	end
end)

local function GamemodeName()
	return (GAMEMODE and GAMEMODE.Name) or engine.ActiveGamemode()
end

-- Collect what the backend needs for a sync: the server's gamemode/map and its players, at most
-- MAX_PLAYERS of them and about MAX_BODY_BYTES (online players first, then the sessions of those
-- who left, oldest first). Returns the JSON body; per reported SteamID64, the whole minutes and
-- the counters it reports (to commit on success); and how many sessions wait for a later sync.
function GravNet.Interval.BuildPayload()
	local now = os.time()
	for _, ply in ipairs(player.GetHumans()) do
		local session = Track(ply)
		if session then
			Accrue(session, now)
			SampleStats(ply, session)
		end
	end

	local entries = {}
	for steam_id, session in pairs(Sessions) do
		local minutes = math.floor(session.pending / 60)
		local counters = {}
		for key, count in pairs(session.counters) do
			counters[key] = math.min(math.floor(count), MAX_COUNTER)
		end
		entries[#entries + 1] = {
			steam_id = steam_id,
			online = session.online == true,
			connected_at = session.connected_at or 0,
			player = {
				steam_id = steam_id,
				name = session.name,
				usergroup = session.usergroup,
				connected_at = session.connected_at,
				minutes = minutes,
				-- Left out when empty: util.TableToJSON writes an empty table as [].
				stats = next(session.stats) and session.stats or nil,
				counters = next(counters) and counters or nil,
			},
			report = { minutes = minutes, counters = counters },
		}
	end
	table.sort(entries, function(a, b)
		if a.online ~= b.online then return a.online end
		if a.connected_at ~= b.connected_at then return a.connected_at < b.connected_at end
		return a.steam_id < b.steam_id
	end)

	local players, reported, bytes = {}, {}, 0
	for _, entry in ipairs(entries) do
		if #players >= MAX_PLAYERS then break end
		local size = #util.TableToJSON(entry.player) + 1
		if #players > 0 and bytes + size > MAX_BODY_BYTES then break end
		players[#players + 1] = entry.player
		reported[entry.steam_id] = entry.report
		bytes = bytes + size
	end

	return {
		gamemode = GamemodeName(),
		map = game.GetMap(),
		players = players,
	}, reported, #entries - #players
end

-- The heartbeat was accepted: subtract the reported playtime and counters, drop finished sessions.
local function Commit(reported)
	for steam_id, report in pairs(reported) do
		local session = Sessions[steam_id]
		if session then
			session.pending = session.pending - report.minutes * 60
			for key, count in pairs(report.counters) do
				local left = (session.counters[key] or 0) - count
				session.counters[key] = left > 0 and left or nil
			end
			if not session.online then Sessions[steam_id] = nil end
		end
	end
end

local function Record(ok, message)
	local changed = not LastSync or LastSync.ok ~= ok or LastSync.message ~= message
	LastSync = { time = os.time(), ok = ok, message = message }
	-- Log failures once rather than every minute while GravNet is down.
	if not ok and changed then GravNet.Util.Warn("Sync failed: " .. message) end
end

-- Run one sync immediately. Also used by the "force interval update" menu option.
--   callback: function(success: boolean, message: string)
function GravNet.Interval.Run(callback)
	callback = callback or function() end

	if not GravNet.Storage.IsLinked() then
		callback(false, "This server is not linked to GravNet.")
		return
	end
	if InFlight then
		callback(false, "A sync is already in progress.")
		return
	end

	InFlight = true
	local payload, reported, waiting = GravNet.Interval.BuildPayload()

	GravNet.HTTP.ServerRequest("POST", "/heartbeat", payload, function(ok, status, data, err, current)
		InFlight = false

		if not current then
			-- Re-linked or unlinked while the request was in flight; the result is stale.
			callback(false, "The link changed during the sync.")
			GravNet.BroadcastStatus()
			return
		end

		if ok then
			Commit(reported)
			if istable(data) and istable(data.server) then
				GravNet.Storage.SetServerName(data.server.name)
			end

			local message = string.format("Synced %d player(s).", #payload.players)
			if waiting > 0 then
				message = message .. string.format(" %d more will be reported in the next syncs.", waiting)
			end
			Record(true, message)

			if AwaitingFirst then
				AwaitingFirst = false
				local name = GravNet.Storage.GetServerName() or ""
				GravNet.Util.Log("First heartbeat accepted")
				GravNet.NotifyAdmins("First heartbeat accepted. \"" .. name .. "\" now shows as Online on GravNet.", true)
			end

			callback(true, message)
		elseif status == 401 then
			GravNet.Link.OnOutOfSync()
			callback(false, "GravNet rejected the server token; the server has been unlinked.")
		else
			Record(false, err or "unknown error")
			callback(false, "Sync failed: " .. (err or "unknown error"))
		end

		GravNet.BroadcastStatus()
	end)
end

-- Start the repeating timer (no-op if not linked).
function GravNet.Interval.Start()
	if not GravNet.Storage.IsLinked() then return end

	timer.Create(TIMER, GravNet.Config.IntervalSeconds, 0, function()
		GravNet.Interval.Run()
	end)
end

function GravNet.Interval.Stop()
	timer.Remove(TIMER)
	AwaitingFirst = false
	LastSync = nil
end

-- Called after a successful link: start the timer and send the first heartbeat now, so the
-- website shows the server as online straight away.
function GravNet.Interval.OnLinked()
	AwaitingFirst = true
	LastSync = nil
	GravNet.Interval.Start()
	GravNet.Interval.Run()
end

hook.Add("InitPostEntity", "GravNet.Interval.Start", function()
	if not GravNet.Storage.IsLinked() then
		GravNet.Util.Log("Not linked. A superadmin can link this server with " .. GravNet.Config.ChatCommand .. ".")
		return
	end

	GravNet.Util.Log(string.format("Linked as server #%d (%s)",
		GravNet.Storage.GetServerId(), GravNet.Storage.GetServerName() or "?"))
	if not GravNet.HTTP.IsConfigured() then
		GravNet.Util.Warn("gravnet_api_url is not set; syncing is paused until it is.")
	end

	GravNet.Interval.Start()
	-- HTTP isn't usable until the server has ticked a few times.
	timer.Simple(5, function() GravNet.Interval.Run() end)
end)
