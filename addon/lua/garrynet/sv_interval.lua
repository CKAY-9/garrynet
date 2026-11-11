-- The GNet interval: every minute, sync server and player information to GarryNet.
-- POST /gnet/server/{SERVER_ID}/heartbeat (see GarryNet.HTTP.ServerRequest).
-- The first successful sync after linking is the server's first heartbeat: the backend fills in
-- gamemode and map from it (docs/SERVERS.md).

GarryNet.Interval = GarryNet.Interval or {}

local TIMER = "GarryNet.Interval"

-- Player sessions by SteamID64. Kept across failed syncs so no playtime is lost while GarryNet
-- is unreachable; a disconnected player's session is dropped once it has been reported.
--   { name, usergroup, connected_at (unix seconds), counted_at, pending (unreported seconds), online,
--     stats (latest gamemode stat values), counters (unreported gamemode counters) }
-- Gamemode stats come from sv_stats.lua.
GarryNet.Interval.Sessions = GarryNet.Interval.Sessions or {}
local Sessions = GarryNet.Interval.Sessions

-- Sessions kept across a Lua refresh may predate the stat fields.
for _, session in pairs(Sessions) do
	session.stats = session.stats or {}
	session.counters = session.counters or {}
end

-- Most a counter can grow by in one heartbeat; the backend caps it there too. The rest waits for
-- the next heartbeat.
local MAX_COUNTER = 1000000

-- True while a heartbeat is in flight. Two at once would report the same playtime twice.
local InFlight = false

-- Set when linking; the next successful heartbeat is announced as the first one.
local AwaitingFirst = false

-- The latest sync attempt: { time = os.time(), ok = bool, message = string } or nil.
local LastSync = nil

function GarryNet.Interval.IsBusy()
	return InFlight
end

function GarryNet.Interval.GetLastSync()
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
	for key, value in pairs(GarryNet.Stats.Sample(ply)) do
		session.stats[key] = value
	end
end

-- Add to one of a player's gamemode counters (see GarryNet.Stats.Count).
function GarryNet.Interval.Count(ply, key, amount)
	if not isnumber(amount) or amount <= 0 then return end
	local session = Track(ply)
	if session then session.counters[key] = (session.counters[key] or 0) + amount end
end

-- Move playtime since the last count into `pending`.
local function Accrue(session, now)
	session.pending = session.pending + math.max(0, now - session.counted_at)
	session.counted_at = now
end

hook.Add("PlayerInitialSpawn", "GarryNet.Interval.Track", function(ply)
	Track(ply)
end)

hook.Add("PlayerDisconnected", "GarryNet.Interval.Track", function(ply)
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

-- Collect what the backend needs for a sync: the server's gamemode/map and its players.
-- Returns the JSON body and, per SteamID64, the whole minutes and the counters it reports (to
-- commit on success).
function GarryNet.Interval.BuildPayload()
	local now = os.time()
	for _, ply in ipairs(player.GetHumans()) do
		local session = Track(ply)
		if session then
			Accrue(session, now)
			SampleStats(ply, session)
		end
	end

	local players, reported = {}, {}
	for steam_id, session in pairs(Sessions) do
		local minutes = math.floor(session.pending / 60)
		local counters = {}
		for key, count in pairs(session.counters) do
			counters[key] = math.min(math.floor(count), MAX_COUNTER)
		end
		players[#players + 1] = {
			steam_id = steam_id,
			name = session.name,
			usergroup = session.usergroup,
			connected_at = session.connected_at,
			minutes = minutes,
			-- Left out when empty: util.TableToJSON writes an empty table as [].
			stats = next(session.stats) and session.stats or nil,
			counters = next(counters) and counters or nil,
		}
		reported[steam_id] = { minutes = minutes, counters = counters }
	end

	return {
		gamemode = GamemodeName(),
		map = game.GetMap(),
		players = players,
	}, reported
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
	-- Log failures once rather than every minute while GarryNet is down.
	if not ok and changed then GarryNet.Util.Warn("Sync failed: " .. message) end
end

-- Run one sync immediately. Also used by the "force interval update" menu option.
--   callback: function(success: boolean, message: string)
function GarryNet.Interval.Run(callback)
	callback = callback or function() end

	if not GarryNet.Storage.IsLinked() then
		callback(false, "This server is not linked to GarryNet.")
		return
	end
	if InFlight then
		callback(false, "A sync is already in progress.")
		return
	end

	InFlight = true
	local payload, reported = GarryNet.Interval.BuildPayload()

	GarryNet.HTTP.ServerRequest("POST", "/heartbeat", payload, function(ok, status, data, err, current)
		InFlight = false

		if not current then
			-- Re-linked or unlinked while the request was in flight; the result is stale.
			callback(false, "The link changed during the sync.")
			GarryNet.BroadcastStatus()
			return
		end

		if ok then
			Commit(reported)
			if istable(data) and istable(data.server) then
				GarryNet.Storage.SetServerName(data.server.name)
			end

			local message = string.format("Synced %d player(s).", #payload.players)
			Record(true, message)

			if AwaitingFirst then
				AwaitingFirst = false
				local name = GarryNet.Storage.GetServerName() or ""
				GarryNet.Util.Log("First heartbeat accepted")
				GarryNet.NotifyAdmins("First heartbeat accepted. \"" .. name .. "\" now shows as Online on GarryNet.", true)
			end

			callback(true, message)
		elseif status == 401 then
			GarryNet.Link.OnOutOfSync()
			callback(false, "GarryNet rejected the server token; the server has been unlinked.")
		else
			Record(false, err or "unknown error")
			callback(false, "Sync failed: " .. (err or "unknown error"))
		end

		GarryNet.BroadcastStatus()
	end)
end

-- Start the repeating timer (no-op if not linked).
function GarryNet.Interval.Start()
	if not GarryNet.Storage.IsLinked() then return end

	timer.Create(TIMER, GarryNet.Config.IntervalSeconds, 0, function()
		GarryNet.Interval.Run()
	end)
end

function GarryNet.Interval.Stop()
	timer.Remove(TIMER)
	AwaitingFirst = false
	LastSync = nil
end

-- Called after a successful link: start the timer and send the first heartbeat now, so the
-- website shows the server as online straight away.
function GarryNet.Interval.OnLinked()
	AwaitingFirst = true
	LastSync = nil
	GarryNet.Interval.Start()
	GarryNet.Interval.Run()
end

hook.Add("InitPostEntity", "GarryNet.Interval.Start", function()
	if not GarryNet.Storage.IsLinked() then
		GarryNet.Util.Log("Not linked. A superadmin can link this server with " .. GarryNet.Config.ChatCommand .. ".")
		return
	end

	GarryNet.Util.Log(string.format("Linked as server #%d (%s)",
		GarryNet.Storage.GetServerId(), GarryNet.Storage.GetServerName() or "?"))
	if not GarryNet.HTTP.IsConfigured() then
		GarryNet.Util.Warn("garrynet_api_url is not set; syncing is paused until it is.")
	end

	GarryNet.Interval.Start()
	-- HTTP isn't usable until the server has ticked a few times.
	timer.Simple(5, function() GarryNet.Interval.Run() end)
end)
