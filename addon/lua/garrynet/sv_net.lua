-- Server side of the client <-> server net messages used by the !gnet menu.
-- Every handler verifies the sender is a superadmin before doing anything.

for _, name in pairs(GarryNet.Net) do
	util.AddNetworkString(name)
end

-- The link status shown in the menu (see sh_status.lua). Never includes the JWT.
function GarryNet.GetStatus()
	local linked = GarryNet.Storage.IsLinked()
	local status = {
		configured = GarryNet.HTTP.IsConfigured(),
		insecure = GarryNet.HTTP.IsInsecure(),
		linked = linked,
		busy = GarryNet.Link.IsBusy() or GarryNet.Interval.IsBusy(),
	}

	if linked then
		status.server_id = GarryNet.Storage.GetServerId()
		status.server_name = GarryNet.Storage.GetServerName() or ""
	end

	local last = GarryNet.Interval.GetLastSync()
	if last then
		status.last_sync = { ago = os.time() - last.time, ok = last.ok, message = last.message }
	end

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
function GarryNet.SendStatus(ply)
	net.Start(GarryNet.Net.Status)
	GarryNet.Status.Write(GarryNet.GetStatus())
	net.Send(ply)
end

-- Send the current status to every superadmin, so open menus stay current.
function GarryNet.BroadcastStatus()
	local admins = Superadmins()
	if #admins > 0 then GarryNet.SendStatus(admins) end
end

-- Tell a player the outcome of an action they requested.
function GarryNet.SendResult(ply, success, message)
	if not IsValid(ply) then return end
	net.Start(GarryNet.Net.Result)
	net.WriteBool(success)
	net.WriteString(message)
	net.Send(ply)
end

local function SendNotice(recipients, message, success)
	net.Start(GarryNet.Net.Notify)
	net.WriteString(message)
	net.WriteBool(success ~= nil)
	net.WriteBool(success == true)
	net.Send(recipients)
end

-- A chat notice for every superadmin. `success` picks the color: true (green), false (red),
-- nil (neutral).
function GarryNet.NotifyAdmins(message, success)
	local admins = Superadmins()
	if #admins == 0 then return end
	SendNotice(admins, message, success)
end

-- Play the store reward sound on one player's client.
function GarryNet.PlayRewardSound(ply)
	if not IsValid(ply) or GarryNet.Config.Store.RewardSound == "" then return end
	net.Start(GarryNet.Net.RewardSound)
	net.Send(ply)
end

-- A chat notice for one player (e.g. their store purchase arrived). Same colors.
function GarryNet.NotifyPlayer(ply, message, success)
	if not IsValid(ply) then return end
	SendNotice(ply, message, success)
end

-- Open the menu on a client.
function GarryNet.OpenMenu(ply)
	net.Start(GarryNet.Net.OpenMenu)
	GarryNet.Status.Write(GarryNet.GetStatus())
	net.Send(ply)
end

-- Superadmins only, and at most one action per second each.
local function IsAllowed(ply)
	if not IsValid(ply) or not ply:IsSuperAdmin() then return false end
	if (ply.GarryNetNextAction or 0) > CurTime() then
		GarryNet.SendResult(ply, false, "Slow down: one action per second.")
		return false
	end
	ply.GarryNetNextAction = CurTime() + 1
	return true
end

local function Who(ply)
	return string.format("%s (%s)", ply:Nick(), ply:SteamID())
end

net.Receive(GarryNet.Net.Link, function(_, ply)
	if not IsAllowed(ply) then return end
	local access_key = net.ReadString()

	GarryNet.Link.Link(access_key, function(success, message)
		GarryNet.SendResult(ply, success, message)
		GarryNet.BroadcastStatus()
		if success then
			GarryNet.Util.Log(Who(ply) .. " linked the server")
			GarryNet.NotifyAdmins(ply:Nick() .. " linked this server: " .. message, true)
		end
	end)
	GarryNet.BroadcastStatus() -- busy
end)

net.Receive(GarryNet.Net.Unlink, function(_, ply)
	if not IsAllowed(ply) then return end

	GarryNet.Link.Unlink(function(success, message)
		GarryNet.SendResult(ply, success, message)
		GarryNet.BroadcastStatus()
		if success then
			GarryNet.Util.Log(Who(ply) .. " unlinked the server")
			GarryNet.NotifyAdmins(ply:Nick() .. " unlinked this server from GarryNet.", nil)
		end
	end)
	GarryNet.BroadcastStatus() -- busy
end)

net.Receive(GarryNet.Net.ForceSync, function(_, ply)
	if not IsAllowed(ply) then return end

	GarryNet.Interval.Run(function(success, message)
		GarryNet.SendResult(ply, success, message)
	end)
	GarryNet.BroadcastStatus() -- busy
end)
