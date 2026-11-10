-- Store delivery (docs/STORE.md "Item Fulfillment"). GarryNet keeps a queue of fulfillments per
-- server; the addon fetches it when a player joins and every minute, applies the rewards of
-- players who are online, and reports back:
--   GET  /gnet/server/{SERVER_ID}/fulfillments[?steam_id=]       what to deliver or take back
--   POST /gnet/server/{SERVER_ID}/fulfillments/{ID}/ack          fulfilled | failed | revoked
--
-- Delivery must never happen twice (money would be paid out again), so a fulfillment is
-- recorded in garrysmod/data/garrynet/store_pending.json once applied and only forgotten after
-- GarryNet confirms the acknowledgement. If the acknowledgement is lost, the next fetch
-- re-acknowledges instead of applying again.

GarryNet.Store = GarryNet.Store or {}
local S = GarryNet.Store

local FILE = GarryNet.Config.DataDir .. "/store_pending.json"

-- Forget local records this old (only errored ones should ever linger).
local PENDING_MAX_AGE = 30 * 24 * 60 * 60

local function Cfg() return GarryNet.Config.Store end

-- Applied in-game but not yet acknowledged, by fulfillment id:
--   { state = "applied" | "errored", at = unix seconds, error = string|nil }
-- "errored" means applying raised a Lua error part-way; it's never retried automatically,
-- since some rewards may already have been given. Admins see it as failed on the website.
local Pending = {}

-- Fulfillment ids being handled right now (an acknowledgement is in flight).
local Busy = {}

-- When each failed fulfillment was last tried this session, to back off between retries.
local LastFailure = {}

-- Ids we've already warned admins about this session.
local Warned = {}

-- True while a server-wide fetch is in flight.
local Fetching = false

-- The latest server-wide fetch: { time, ok, message } or nil.
local LastSync = nil

local function Save()
	local list = {}
	for id, p in pairs(Pending) do
		list[#list + 1] = { id = id, state = p.state, at = p.at, error = p.error }
	end
	if not file.IsDir(GarryNet.Config.DataDir, "DATA") then file.CreateDir(GarryNet.Config.DataDir) end
	file.Write(FILE, util.TableToJSON(list))
end

local function Load()
	local raw = file.Read(FILE, "DATA")
	if not raw or raw == "" then return end
	local list = util.JSONToTable(raw)
	if not istable(list) then
		GarryNet.Util.Warn("Ignoring unreadable " .. FILE)
		return
	end
	local now = os.time()
	for _, p in ipairs(list) do
		local id = istable(p) and tonumber(p.id)
		if id and (p.state == "applied" or p.state == "errored") and now - (tonumber(p.at) or 0) < PENDING_MAX_AGE then
			Pending[id] = { state = p.state, at = tonumber(p.at), error = p.error }
		end
	end
end

function S.GetLastSync()
	return LastSync
end

-- Tell GarryNet what happened to fulfillment `id`. Clears the local record once an "applied"
-- delivery is confirmed.
local function Ack(id, result, error_message)
	Busy[id] = true
	local body = { result = result, error = error_message }
	GarryNet.HTTP.ServerRequest("POST", "/fulfillments/" .. id .. "/ack", body, function(ok, status, _, err, current)
		Busy[id] = nil
		if not current then return end
		if status == 401 then
			GarryNet.Link.OnOutOfSync()
			return
		end
		-- 404: GarryNet no longer has it (e.g. the server was re-registered); nothing left to report.
		if ok or status == 404 then
			if Pending[id] and Pending[id].state == "applied" then
				Pending[id] = nil
				Save()
			end
			return
		end
		-- Kept in Pending; the next fetch re-acknowledges.
		GarryNet.Util.Warn(string.format("Couldn't report store purchase #%d as %s: %s", id, result, tostring(err)))
	end)
end

local function DescribeAll(rewards)
	local parts = {}
	for _, r in ipairs(rewards) do parts[#parts + 1] = GarryNet.Rewards.Describe(r) end
	return table.concat(parts, ", ")
end

-- Whether a failed fulfillment is due for another try: waits 1, 2, 4, ... minutes after each
-- failure (up to MaxRetryMinutes), so a fix on the server side gets a chance before GarryNet
-- stops offering it.
local function RetryDue(f)
	local last = LastFailure[f.id]
	if not last then return true end
	local attempts = tonumber(f.attempts) or 1
	local wait = math.min(60 * 2 ^ math.max(0, attempts - 1), Cfg().MaxRetryMinutes * 60)
	return os.time() - last >= wait
end

local function Failed(f, ply, reason)
	LastFailure[f.id] = os.time()
	Ack(f.id, "failed", reason)
	if not Warned[f.id] then
		Warned[f.id] = true
		local message = string.format("Store purchase #%d for %s couldn't be delivered: %s", f.id, f.steam_id, reason)
		GarryNet.Util.Warn(message)
		GarryNet.NotifyAdmins(message, false)
		if ply then
			GarryNet.NotifyPlayer(ply, "Your store purchase couldn't be delivered yet (" .. reason
				.. "). Staff have been told; it will be retried.", false)
		end
	end
end

-- Handle one fulfillment from the queue. Returns what happened: "delivered", "revoked",
-- "failed", "offline", "waiting" or "skipped".
local function Handle(f)
	local id = f.id
	if Busy[id] then return "skipped" end

	if f.status == "REVOKED" then
		GarryNet.Rewards.Revoke(f)
		if Pending[id] then
			Pending[id] = nil
			Save()
		end
		local ply = player.GetBySteamID64(f.steam_id)
		if IsValid(ply) then
			GarryNet.NotifyPlayer(ply, "A refunded store purchase was taken back: " .. DescribeAll(f.rewards) .. ".", nil)
		end
		GarryNet.Util.Log(string.format("Store: revoked purchase #%d for %s", id, f.steam_id))
		Ack(id, "revoked")
		return "revoked"
	end

	local pending = Pending[id]
	if pending and pending.state == "applied" then
		-- Applied before, but GarryNet never heard about it.
		Ack(id, "fulfilled")
		return "delivered"
	end
	if pending and pending.state == "errored" then return "skipped" end

	local ply = player.GetBySteamID64(f.steam_id)
	if not IsValid(ply) then return "offline" end
	if f.status == "FAILED" and not RetryDue(f) then return "waiting" end

	local verdict, reason = GarryNet.Rewards.Validate(f.rewards, ply)
	if verdict == "wait" then return "waiting" end
	if verdict ~= "ok" then
		Failed(f, ply, reason)
		return "failed"
	end

	local ok, err = pcall(GarryNet.Rewards.Apply, f, ply)
	if not ok then
		Pending[id] = { state = "errored", at = os.time(), error = tostring(err) }
		Save()
		Failed(f, ply, "error while applying the rewards: " .. tostring(err))
		return "failed"
	end

	Pending[id] = { state = "applied", at = os.time() }
	Save()
	LastFailure[id] = nil
	GarryNet.Util.Log(string.format("Store: delivered purchase #%d to %s (%s)", id, ply:Nick(), f.steam_id))
	GarryNet.NotifyPlayer(ply, "Thanks for your purchase! You received: " .. DescribeAll(f.rewards) .. ".", true)
	Ack(id, "fulfilled")
	return "delivered"
end

local function IsFulfillment(f)
	return istable(f) and isnumber(f.id) and isstring(f.steam_id) and istable(f.rewards) and isstring(f.status)
end

-- Fetch the queue and handle it. `steam_id` limits it to one player (used when they join).
--   callback: function(success: boolean, message: string)
function S.Sync(steam_id, callback)
	callback = callback or function() end

	if not GarryNet.Storage.IsLinked() then
		callback(false, "This server is not linked to GarryNet.")
		return
	end
	if not steam_id then
		if Fetching then
			callback(false, "A store sync is already in progress.")
			return
		end
		Fetching = true
	end

	local route = "/fulfillments" .. (steam_id and ("?steam_id=" .. steam_id) or "")
	GarryNet.HTTP.ServerRequest("GET", route, nil, function(ok, status, data, err, current)
		if not steam_id then Fetching = false end

		if not current then
			callback(false, "The link changed during the store sync.")
			return
		end
		if status == 401 then
			GarryNet.Link.OnOutOfSync()
			callback(false, "GarryNet rejected the server token; the server has been unlinked.")
			return
		end

		local message
		if not ok or not istable(data) then
			message = "Store sync failed: " .. (err or "unexpected response from GarryNet")
			if not steam_id then
				local changed = not LastSync or LastSync.ok or LastSync.message ~= message
				LastSync = { time = os.time(), ok = false, message = message }
				if changed then GarryNet.Util.Warn(message) end
			end
			callback(false, message)
			return
		end

		local counts = {}
		for _, f in ipairs(data) do
			if IsFulfillment(f) then
				local outcome = Handle(f)
				counts[outcome] = (counts[outcome] or 0) + 1
			end
		end

		local parts = {}
		for _, outcome in ipairs({ "delivered", "revoked", "failed", "offline", "waiting" }) do
			if counts[outcome] then parts[#parts + 1] = counts[outcome] .. " " .. outcome end
		end
		message = #parts > 0 and ("Store: " .. table.concat(parts, ", ") .. ".") or "Store: nothing to deliver."
		if not steam_id then LastSync = { time = os.time(), ok = true, message = message } end
		callback(true, message)
	end)
end

-- Every minute, and shortly after the map loads.
timer.Create("GarryNet.Store.Poll", GarryNet.Config.Store.PollSeconds, 0, function()
	if GarryNet.Storage.IsLinked() and GarryNet.HTTP.IsConfigured() then S.Sync() end
end)

hook.Add("InitPostEntity", "GarryNet.Store.Start", function()
	timer.Simple(10, function()
		if GarryNet.Storage.IsLinked() and GarryNet.HTTP.IsConfigured() then S.Sync() end
	end)
end)

-- When a player joins, deliver what they bought while away. Delayed so the gamemode (e.g.
-- DarkRP's money) and the admin mod have loaded the player first.
hook.Add("PlayerInitialSpawn", "GarryNet.Store.Join", function(ply)
	if ply:IsBot() then return end
	local steam_id = ply:SteamID64()
	timer.Simple(GarryNet.Config.Store.JoinDelay + 1, function()
		if IsValid(ply) and GarryNet.Storage.IsLinked() and GarryNet.HTTP.IsConfigured() then S.Sync(steam_id) end
	end)
end)

Load()
