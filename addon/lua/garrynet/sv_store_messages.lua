-- Store chat messages (docs/ADDON.md "Store chat messages"): when a superadmin turns them on in the
-- !gnet menu, every player gets a chat message advertising the store every few minutes. The
-- settings are saved in garrysmod/data/garrynet/store_messages.json.
--
-- Before each message the addon asks GarryNet whether the server still has an enabled store
-- (GET /gnet/server/{SERVER_ID}/store), so a disabled or missing store is never advertised.

GarryNet.StoreMessages = GarryNet.StoreMessages or {}
local M = GarryNet.StoreMessages

local FILE = GarryNet.Config.DataDir .. "/store_messages.json"
local TIMER = "GarryNet.StoreMessages"

local function Cfg() return GarryNet.Config.StoreMessages end

local Settings = {
	enabled = false,
	minutes = Cfg().DefaultMinutes,
	message = Cfg().DefaultMessage,
}

-- Why the last message wasn't sent, or nil if it was (or none is due yet).
local Skipped = nil

-- True while asking GarryNet about the store.
local Checking = false

-- Settings, cleaned up; or nil and why they're refused.
local function Validate(enabled, minutes, message)
	local cfg = Cfg()
	minutes = math.floor(tonumber(minutes) or 0)
	if minutes < cfg.MinMinutes or minutes > cfg.MaxMinutes then
		return nil, string.format("The interval must be %d to %d minutes.", cfg.MinMinutes, cfg.MaxMinutes)
	end
	message = string.Trim((string.gsub(isstring(message) and message or "", "[\r\n\t]+", " ")))
	if message == "" then return nil, "The message can't be empty." end
	if utf8.len(message) == nil then return nil, "The message isn't valid text." end
	if utf8.len(message) > cfg.MaxLength then
		return nil, "The message can be at most " .. cfg.MaxLength .. " characters."
	end
	return { enabled = enabled == true, minutes = minutes, message = message }
end

local function SetSkipped(reason)
	if reason == Skipped then return end
	Skipped = reason
	GarryNet.BroadcastStatus() -- open !gnet menus show it
end

-- Send the message to everyone, if the server has a store to advertise.
local function Send()
	if Checking or #player.GetHumans() == 0 then return end
	if not GarryNet.Storage.IsLinked() or not GarryNet.HTTP.IsConfigured() then
		SetSkipped("this server isn't linked to GarryNet.")
		return
	end

	Checking = true
	GarryNet.HTTP.ServerRequest("GET", "/store", nil, function(ok, status, data, err, current)
		Checking = false
		if current and status == 401 then GarryNet.Link.OnOutOfSync() end
		if not ok or not current or not istable(data) then
			SetSkipped("GarryNet couldn't be reached (" .. (err or "unexpected response") .. ").")
			return
		end
		if not istable(data.store) then
			SetSkipped("this server has no store, or it's disabled, on GarryNet.")
			return
		end

		GarryNet.NotifyAll(Settings.message)
		SetSkipped(nil)
	end)
end

-- (Re)start the timer for the current settings; stop it when they're off.
local function Schedule()
	if Settings.enabled then
		timer.Create(TIMER, Settings.minutes * 60, 0, Send)
	else
		timer.Remove(TIMER)
	end
end

local function Load()
	local raw = file.Read(FILE, "DATA")
	if not raw or raw == "" then return end
	local data = util.JSONToTable(raw)
	local settings = istable(data) and Validate(data.enabled, data.minutes, data.message)
	if not settings then
		GarryNet.Util.Warn("Ignoring unreadable " .. FILE)
		return
	end
	Settings = settings
end

-- The settings and their state, for the !gnet menu (see sh_status.lua).
function M.GetStatus()
	local left = timer.TimeLeft(TIMER)
	return {
		enabled = Settings.enabled,
		minutes = Settings.minutes,
		message = Settings.message,
		next_in = Settings.enabled and left and math.max(0, left) or nil,
		skipped = Skipped or "",
	}
end

-- Replace and save the settings. The countdown restarts only when it's turned on or the
-- interval changes, so fixing a typo doesn't put off the next message.
--   returns: ok, message
function M.Save(enabled, minutes, message)
	local settings, err = Validate(enabled, minutes, message)
	if not settings then return false, err end

	local reschedule = settings.enabled ~= Settings.enabled or settings.minutes ~= Settings.minutes
	Settings = settings
	if not file.IsDir(GarryNet.Config.DataDir, "DATA") then file.CreateDir(GarryNet.Config.DataDir) end
	file.Write(FILE, util.TableToJSON(Settings))
	if reschedule then
		Skipped = nil
		Schedule()
	end

	if not Settings.enabled then return true, "Store chat messages saved. They're off." end
	return true, string.format("Store chat messages saved. One goes out every %d minute%s.",
		Settings.minutes, Settings.minutes == 1 and "" or "s")
end

Load()
Schedule()
