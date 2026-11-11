-- Ban enforcement (docs/ADDON.md "Ban Enforcement"). When the community turns on "Keep banned
-- users off game servers" on the website, players banned on GarryNet are refused when they
-- connect and kicked if they're already playing:
--   GET /gnet/server/{SERVER_ID}/bans   -> { enforce, appeal_url, bans = { { steam_id, reason, ends } } }
--
-- CheckPassword has to answer straight away, so the list is fetched every minute rather than
-- per connection; a new ban takes up to a minute to apply. The last list is kept in
-- garrysmod/data/garrynet/bans.json and keeps being enforced while GarryNet is unreachable,
-- including across restarts. Bans that end are dropped locally on time.

GarryNet.Bans = GarryNet.Bans or {}
local B = GarryNet.Bans

local FILE = GarryNet.Config.DataDir .. "/bans.json"

-- The latest list from GarryNet. `bans` is keyed by SteamID64: { reason = string, ends = unix seconds|nil }.
local List = { enforce = false, appeal_url = "", bans = {}, count = 0 }

-- True while a fetch is in flight.
local Fetching = false

-- The latest fetch: { time, ok, message } or nil.
local LastSync = nil

function B.GetLastSync()
	return LastSync
end

-- Whether banned players are being refused, and how many bans are known.
function B.IsEnforcing()
	return List.enforce, List.count
end

-- Build the list from the backend's response (or the cached copy of it).
local function Parse(data)
	local list = {
		enforce = data.enforce == true,
		appeal_url = isstring(data.appeal_url) and data.appeal_url or "",
		bans = {},
		count = 0,
	}
	if list.enforce and istable(data.bans) then
		for _, ban in ipairs(data.bans) do
			-- SteamID64s are strings: they don't fit in a Lua number.
			if istable(ban) and isstring(ban.steam_id) and string.match(ban.steam_id, "^%d+$") then
				list.bans[ban.steam_id] = {
					reason = isstring(ban.reason) and ban.reason or "",
					ends = tonumber(ban.ends),
				}
				list.count = list.count + 1
			end
		end
	end
	return list
end

-- Saved as an array, as GarryNet sent it: util.JSONToTable would turn SteamID64 keys into numbers.
local function Save()
	local bans = {}
	for steam_id, ban in pairs(List.bans) do
		bans[#bans + 1] = { steam_id = steam_id, reason = ban.reason, ends = ban.ends }
	end
	if not file.IsDir(GarryNet.Config.DataDir, "DATA") then file.CreateDir(GarryNet.Config.DataDir) end
	file.Write(FILE, util.TableToJSON({ enforce = List.enforce, appeal_url = List.appeal_url, bans = bans }))
end

local function Load()
	local raw = file.Read(FILE, "DATA")
	if not raw or raw == "" then return end
	local data = util.JSONToTable(raw)
	if not istable(data) then
		GarryNet.Util.Warn("Ignoring unreadable " .. FILE)
		return
	end
	List = Parse(data)
end

-- The ban keeping `steam_id` (a SteamID64) off this server, or nil.
function B.Find(steam_id)
	if not List.enforce or not GarryNet.Storage.IsLinked() then return nil end
	local ban = List.bans[tostring(steam_id)]
	if not ban or (ban.ends and ban.ends <= os.time()) then return nil end
	return ban
end

-- What a banned player sees when refused or kicked.
function B.Message(ban)
	local lines = { "You are banned from this community." }
	if ban.reason ~= "" then lines[#lines + 1] = "Reason: " .. ban.reason end
	lines[#lines + 1] = ban.ends and ("Ends: " .. os.date("!%Y-%m-%d %H:%M UTC", ban.ends)) or "This ban is permanent."
	if List.appeal_url ~= "" then lines[#lines + 1] = "Appeal at " .. List.appeal_url end
	return table.concat(lines, "\n")
end

-- Kick anyone playing who is banned now (banned while on the server, or enforcement just turned on).
local function KickBanned()
	for _, ply in ipairs(player.GetHumans()) do
		local ban = B.Find(ply:SteamID64())
		if ban then
			GarryNet.Util.Log(string.format("Kicked %s (%s): banned on GarryNet", ply:Nick(), ply:SteamID64()))
			ply:Kick(B.Message(ban))
		end
	end
end

-- Fetch the ban list, then kick banned players who are online.
--   callback: function(success: boolean, message: string)
function B.Sync(callback)
	callback = callback or function() end

	if not GarryNet.Storage.IsLinked() then
		callback(false, "This server is not linked to GarryNet.")
		return
	end
	if Fetching then
		callback(false, "A ban list sync is already in progress.")
		return
	end

	Fetching = true
	GarryNet.HTTP.ServerRequest("GET", "/bans", nil, function(ok, status, data, err, current)
		Fetching = false

		if not current then
			callback(false, "The link changed during the ban list sync.")
			return
		end
		if status == 401 then
			GarryNet.Link.OnOutOfSync()
			callback(false, "GarryNet rejected the server token; the server has been unlinked.")
			return
		end

		local message
		if not ok or not istable(data) then
			-- Keep enforcing the last list we had.
			message = "Ban list sync failed: " .. (err or "unexpected response from GarryNet")
			local changed = not LastSync or LastSync.ok or LastSync.message ~= message
			LastSync = { time = os.time(), ok = false, message = message }
			if changed then GarryNet.Util.Warn(message) end
			callback(false, message)
			return
		end

		local was_enforcing = List.enforce
		List = Parse(data)
		Save()
		if List.enforce ~= was_enforcing then
			GarryNet.Util.Log(List.enforce and "Players banned on GarryNet are now kept off this server."
				or "Players banned on GarryNet are no longer kept off this server.")
		end
		KickBanned()

		message = List.enforce and string.format("Bans: enforcing %d.", List.count) or "Bans: not enforced."
		LastSync = { time = os.time(), ok = true, message = message }
		callback(true, message)
	end)
end

-- Forget the list, e.g. when the server is unlinked: an old list mustn't be enforced forever.
function B.Clear()
	List = { enforce = false, appeal_url = "", bans = {}, count = 0 }
	LastSync = nil
	if file.Exists(FILE, "DATA") then file.Delete(FILE) end
end

-- Refuse banned players before they load in. Returning nothing leaves the decision (e.g. the
-- server password) to other hooks and the default.
hook.Add("CheckPassword", "GarryNet.Bans", function(steam_id, _, _, _, name)
	local ban = B.Find(steam_id)
	if ban then
		GarryNet.Util.Log(string.format("Refused %s (%s): banned on GarryNet", tostring(name), tostring(steam_id)))
		return false, B.Message(ban)
	end
end)

-- Every minute, and shortly after the map loads.
timer.Create("GarryNet.Bans.Poll", GarryNet.Config.IntervalSeconds, 0, function()
	if GarryNet.Storage.IsLinked() and GarryNet.HTTP.IsConfigured() then B.Sync() end
end)

hook.Add("InitPostEntity", "GarryNet.Bans.Start", function()
	-- HTTP isn't usable until the server has ticked a few times.
	timer.Simple(5, function()
		if GarryNet.Storage.IsLinked() and GarryNet.HTTP.IsConfigured() then B.Sync() end
	end)
end)

Load()
