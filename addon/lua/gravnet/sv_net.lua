-- Server side of the client <-> server net messages used by the !gnet menu.
-- Every handler verifies the sender is a superadmin before doing anything.

for _, name in pairs(GravNet.Net) do
	util.AddNetworkString(name)
end

-- The link status shown in the menu (see sh_status.lua). Never includes the JWT.
function GravNet.GetStatus()
	local linked = GravNet.Storage.IsLinked()
	local status = {
		configured = GravNet.HTTP.IsConfigured(),
		-- What's wrong with gravnet_api_url's security, if anything (see sv_http.lua).
		insecure = GravNet.HTTP.InsecureMessage() or false,
		linked = linked,
		busy = GravNet.Link.IsBusy() or GravNet.Interval.IsBusy(),
	}

	if linked then
		status.server_id = GravNet.Storage.GetServerId()
		status.server_name = GravNet.Storage.GetServerName() or ""
	end

	local last = GravNet.Interval.GetLastSync()
	if last then
		status.last_sync = { ago = os.time() - last.time, ok = last.ok, message = last.message }
	end

	status.store_messages = GravNet.StoreMessages.GetStatus()

	return status
end

local function Superadmins()
	local list = {}
	for _, ply in ipairs(player.GetHumans()) do
		if ply:IsSuperAdmin() then list[#list + 1] = ply end
	end
	return list
end

-- Send the current link status to a player (or a list of players).
function GravNet.SendStatus(ply)
	net.Start(GravNet.Net.Status)
	GravNet.Status.Write(GravNet.GetStatus())
	net.Send(ply)
end

-- Send the current status to every superadmin, so open menus stay current.
function GravNet.BroadcastStatus()
	local admins = Superadmins()
	if #admins > 0 then GravNet.SendStatus(admins) end
end

-- Tell a player the outcome of an action they requested.
function GravNet.SendResult(ply, success, message)
	if not IsValid(ply) then return end
	net.Start(GravNet.Net.Result)
	net.WriteBool(success)
	net.WriteString(message)
	net.Send(ply)
end

local function SendNotice(recipients, message, success)
	net.Start(GravNet.Net.Notify)
	net.WriteString(message)
	net.WriteBool(success ~= nil)
	net.WriteBool(success == true)
	net.Send(recipients)
end

-- A chat notice for every superadmin. `success` picks the color: true (green), false (red),
-- nil (neutral).
function GravNet.NotifyAdmins(message, success)
	local admins = Superadmins()
	if #admins == 0 then return end
	SendNotice(admins, message, success)
end

-- Play the store reward sound on one player's client.
function GravNet.PlayRewardSound(ply)
	if not IsValid(ply) or GravNet.Config.Store.RewardSound == "" then return end
	net.Start(GravNet.Net.RewardSound)
	net.Send(ply)
end

-- A chat notice for one player (e.g. their store purchase arrived). Same colors.
function GravNet.NotifyPlayer(ply, message, success)
	if not IsValid(ply) then return end
	SendNotice(ply, message, success)
end

-- A neutral chat notice for every player (e.g. the store chat message).
function GravNet.NotifyAll(message)
	local humans = player.GetHumans()
	if #humans == 0 then return end
	SendNotice(humans, message, nil)
end

-- Open the menu on a client.
function GravNet.OpenMenu(ply)
	net.Start(GravNet.Net.OpenMenu)
	GravNet.Status.Write(GravNet.GetStatus())
	net.Send(ply)
end

-- Superadmins only, and at most one action per second each.
local function IsAllowed(ply)
	if not IsValid(ply) or not ply:IsSuperAdmin() then return false end
	if (ply.GravNetNextAction or 0) > CurTime() then
		GravNet.SendResult(ply, false, "Slow down: one action per second.")
		return false
	end
	ply.GravNetNextAction = CurTime() + 1
	return true
end

local function Who(ply)
	return string.format("%s (%s)", ply:Nick(), ply:SteamID())
end

net.Receive(GravNet.Net.Link, function(_, ply)
	if not IsAllowed(ply) then return end
	local access_key = net.ReadString()

	GravNet.Link.Link(access_key, function(success, message)
		GravNet.SendResult(ply, success, message)
		GravNet.BroadcastStatus()
		if success then
			GravNet.Util.Log(Who(ply) .. " linked the server")
			GravNet.NotifyAdmins(ply:Nick() .. " linked this server: " .. message, true)
		end
	end)
	GravNet.BroadcastStatus() -- busy
end)

net.Receive(GravNet.Net.Unlink, function(_, ply)
	if not IsAllowed(ply) then return end

	GravNet.Link.Unlink(function(success, message)
		GravNet.SendResult(ply, success, message)
		GravNet.BroadcastStatus()
		if success then
			GravNet.Util.Log(Who(ply) .. " unlinked the server")
			GravNet.NotifyAdmins(ply:Nick() .. " unlinked this server from GravNet.", nil)
		end
	end)
	GravNet.BroadcastStatus() -- busy
end)

net.Receive(GravNet.Net.SaveStoreMessages, function(_, ply)
	if not IsAllowed(ply) then return end
	local enabled = net.ReadBool()
	local minutes = net.ReadUInt(16)
	local message = net.ReadString()

	local success, result = GravNet.StoreMessages.Save(enabled, minutes, message)
	GravNet.SendResult(ply, success, result)
	if success then
		GravNet.Util.Log(Who(ply) .. " saved the store chat messages: " .. (enabled and "on" or "off")
			.. ", every " .. minutes .. " min")
		GravNet.BroadcastStatus()
	end
end)

net.Receive(GravNet.Net.ForceSync, function(_, ply)
	if not IsAllowed(ply) then return end

	GravNet.Interval.Run(function(success, message)
		GravNet.SendResult(ply, success, message)
	end)
	GravNet.BroadcastStatus() -- busy
end)
