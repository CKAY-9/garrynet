-- Store delivery (docs/STORE.md "Item Fulfillment"). GravNet keeps a queue of fulfillments per
-- server; the addon fetches it when a player joins and every minute, applies the rewards of
-- players who are online, and reports back:
--   GET  /gnet/server/{SERVER_ID}/fulfillments[?steam_id=]       what to deliver or take back
--   POST /gnet/server/{SERVER_ID}/fulfillments/{ID}/ack          fulfilled | failed | revoked
--
-- Delivery must never happen twice (money would be paid out again), so a fulfillment is
-- recorded in garrysmod/data/gravnet/store_pending.json once applied and only forgotten after
-- GravNet confirms the acknowledgement. If the acknowledgement is lost, the next fetch
-- re-acknowledges instead of applying again.

GravNet.Store = GravNet.Store or {}
local S = GravNet.Store

local FILE = GravNet.Config.DataDir .. "/store_pending.json"

-- Forget local records this old (only errored ones should ever linger).
local PENDING_MAX_AGE = 30 * 24 * 60 * 60

local function Cfg() return GravNet.Config.Store end

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
	if not file.IsDir(GravNet.Config.DataDir, "DATA") then file.CreateDir(GravNet.Config.DataDir) end
	file.Write(FILE, util.TableToJSON(list))
end

local function Load()
	local raw = file.Read(FILE, "DATA")
	if not raw or raw == "" then return end
	local list = util.JSONToTable(raw)
	if not istable(list) then
		GravNet.Util.Warn("Ignoring unreadable " .. FILE)
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

-- Tell GravNet what happened to fulfillment `id`. Clears the local record once an "applied"
-- delivery is confirmed. `money_reclaimed` is the DarkRP money taken back, for "revoked".
local function Ack(id, result, error_message, money_reclaimed)
	Busy[id] = true
	local body = { result = result, error = error_message, money_reclaimed = money_reclaimed }
	GravNet.HTTP.ServerRequest("POST", "/fulfillments/" .. id .. "/ack", body, function(ok, status, _, err, current)
		Busy[id] = nil
		if not current then return end
		if status == 401 then
			GravNet.Link.OnOutOfSync()
			return
		end
		-- 404: GravNet no longer has it (e.g. the server was re-registered); nothing left to report.
		if ok or status == 404 then
			if Pending[id] and Pending[id].state == "applied" then
				Pending[id] = nil
				Save()
			end
			return
		end
		-- Kept in Pending; the next fetch re-acknowledges.
		GravNet.Util.Warn(string.format("Couldn't report store purchase #%d as %s: %s", id, result, tostring(err)))
	end)
end

local function DescribeAll(rewards)
	local parts = {}
	for _, r in ipairs(rewards) do parts[#parts + 1] = GravNet.Rewards.Describe(r) end
	return table.concat(parts, ", ")
end

-- What revoking took back: the usergroups and weapons, and `money` (the DarkRP money taken).
-- "" if nothing.
local function DescribeTaken(rewards, money)
	local parts = {}
	for _, r in ipairs(rewards) do
		if r.type ~= "darkrp_money" then parts[#parts + 1] = GravNet.Rewards.Describe(r) end
	end
	if money and money > 0 then
		parts[#parts + 1] = GravNet.Rewards.Describe({ type = "darkrp_money", amount = money })
	end
	return table.concat(parts, ", ")
end

-- Whether a failed fulfillment is due for another try: waits 1, 2, 4, ... minutes after each
-- failure (up to MaxRetryMinutes), so a fix on the server side gets a chance before GravNet
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
		GravNet.Util.Warn(message)
		GravNet.NotifyAdmins(message, false)
		if ply then
			GravNet.NotifyPlayer(ply, "Your store purchase couldn't be delivered yet (" .. reason
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
		-- GravNet offers revoked purchases even when it never heard of a delivery: the rewards
		-- may have been applied here with the "fulfilled" report lost before the refund.
		local pending = Pending[id]
		local delivered = f.fulfilled_at ~= nil or pending ~= nil
		GravNet.Rewards.Revoke(f)

		-- The store takes delivered money back (`deduct_money`), which needs the player online
		-- with their money loaded. Until then the usergroups and weapons are already gone, and
		-- the revocation stays unacknowledged so it's offered again.
		-- A delivery that errored part-way may not have paid the money, so it's left alone.
		local paid = f.fulfilled_at ~= nil or (pending ~= nil and pending.state == "applied")
		local money = (paid and f.deduct_money == true) and GravNet.Rewards.MoneyIn(f.rewards) or 0
		local taken = nil
		if money > 0 then
			local verdict
			verdict, taken = GravNet.Rewards.ReclaimMoney(f, money)
			if verdict == "wait" then return "offline" end
		end

		if pending then
			Pending[id] = nil
			Save()
		end
		if delivered then
			local ply = player.GetBySteamID64(f.steam_id)
			local described = DescribeTaken(f.rewards, taken)
			if IsValid(ply) and described ~= "" then
				GravNet.NotifyPlayer(ply, "A refunded store purchase was taken back: " .. described .. ".", nil)
			end
			GravNet.Util.Log(string.format("Store: revoked purchase #%d for %s", id, f.steam_id))
		end
		-- Report the lost delivery too, so the website shows the rewards as delivered and then
		-- removed. The two reports can arrive in either order.
		if pending and pending.state == "applied" and f.fulfilled_at == nil then Ack(id, "fulfilled") end
		Ack(id, "revoked", nil, (taken and taken > 0) and taken or nil)
		return delivered and "revoked" or "skipped"
	end

	local pending = Pending[id]
	if pending and pending.state == "applied" then
		-- Applied before, but GravNet never heard about it.
		Ack(id, "fulfilled")
		return "delivered"
	end
	if pending and pending.state == "errored" then return "skipped" end

	local ply = player.GetBySteamID64(f.steam_id)
	if not IsValid(ply) then return "offline" end
	if f.status == "FAILED" and not RetryDue(f) then return "waiting" end

	local verdict, reason = GravNet.Rewards.Validate(f.rewards, ply)
	if verdict == "wait" then return "waiting" end
	if verdict ~= "ok" then
		Failed(f, ply, reason)
		return "failed"
	end

	local ok, err = pcall(GravNet.Rewards.Apply, f, ply)
	if not ok then
		Pending[id] = { state = "errored", at = os.time(), error = tostring(err) }
		Save()
		Failed(f, ply, "error while applying the rewards: " .. tostring(err))
		return "failed"
	end

	Pending[id] = { state = "applied", at = os.time() }
	Save()
	LastFailure[id] = nil
	GravNet.Util.Log(string.format("Store: delivered purchase #%d to %s (%s)", id, ply:Nick(), f.steam_id))
	GravNet.NotifyPlayer(ply, "Thanks for your purchase! You received: " .. DescribeAll(f.rewards) .. ".", true)
	GravNet.PlayRewardSound(ply)
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

	if not GravNet.Storage.IsLinked() then
		callback(false, "This server is not linked to GravNet.")
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
	GravNet.HTTP.ServerRequest("GET", route, nil, function(ok, status, data, err, current)
		if not steam_id then Fetching = false end

		if not current then
			callback(false, "The link changed during the store sync.")
			return
		end
		if status == 401 then
			GravNet.Link.OnOutOfSync()
			callback(false, "GravNet rejected the server token; the server has been unlinked.")
			return
		end

		local message
		if not ok or not istable(data) then
			message = "Store sync failed: " .. (err or "unexpected response from GravNet")
			if not steam_id then
				local changed = not LastSync or LastSync.ok or LastSync.message ~= message
				LastSync = { time = os.time(), ok = false, message = message }
				if changed then GravNet.Util.Warn(message) end
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
timer.Create("GravNet.Store.Poll", GravNet.Config.Store.PollSeconds, 0, function()
	if GravNet.Storage.IsLinked() and GravNet.HTTP.IsConfigured() then S.Sync() end
end)

hook.Add("InitPostEntity", "GravNet.Store.Start", function()
	timer.Simple(10, function()
		if GravNet.Storage.IsLinked() and GravNet.HTTP.IsConfigured() then S.Sync() end
	end)
end)

-- When a player joins, deliver what they bought while away. Delayed so the gamemode (e.g.
-- DarkRP's money) and the admin mod have loaded the player first.
hook.Add("PlayerInitialSpawn", "GravNet.Store.Join", function(ply)
	if ply:IsBot() then return end
	local steam_id = ply:SteamID64()
	timer.Simple(GravNet.Config.Store.JoinDelay + 1, function()
		if IsValid(ply) and GravNet.Storage.IsLinked() and GravNet.HTTP.IsConfigured() then S.Sync(steam_id) end
	end)
end)

Load()
