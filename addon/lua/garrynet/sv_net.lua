-- Server side of the client <-> server net messages used by the !gnet menu.
-- Every handler must verify the sender is a superadmin before doing anything.

for _, name in pairs(GarryNet.Net) do
	util.AddNetworkString(name)
end

-- Send the current link status to a player (true/false linked, plus anything the menu displays).
function GarryNet.SendStatus(ply)
	-- TODO: net.Start(GarryNet.Net.Status); write linked state (never the JWT); net.Send(ply)
end

-- Tell a player the outcome of an action they requested.
function GarryNet.SendResult(ply, success, message)
	-- TODO: net.Start(GarryNet.Net.Result); net.WriteBool(success); net.WriteString(message); net.Send(ply)
end

-- Open the menu on a client.
function GarryNet.OpenMenu(ply)
	-- TODO: net.Start(GarryNet.Net.OpenMenu); write status; net.Send(ply)
end

local function IsAllowed(ply)
	return IsValid(ply) and ply:IsSuperAdmin()
end

net.Receive(GarryNet.Net.Link, function(_, ply)
	if not IsAllowed(ply) then return end
	-- TODO: read access key, call GarryNet.Link.Link, reply with SendResult + SendStatus.
end)

net.Receive(GarryNet.Net.Unlink, function(_, ply)
	if not IsAllowed(ply) then return end
	-- TODO: call GarryNet.Link.Unlink, reply with SendResult + SendStatus.
end)

net.Receive(GarryNet.Net.ForceSync, function(_, ply)
	if not IsAllowed(ply) then return end
	-- TODO: call GarryNet.Interval.Run, reply with SendResult.
end)
