-- Client side of the net messages. Keeps the latest status for the menu to render.

GarryNet.ClientState = GarryNet.ClientState or {
	linked = false,
}

net.Receive(GarryNet.Net.OpenMenu, function()
	-- TODO: read status, update GarryNet.ClientState, call GarryNet.Menu.Open().
end)

net.Receive(GarryNet.Net.Status, function()
	-- TODO: read status, update GarryNet.ClientState, refresh the menu if it is open.
end)

net.Receive(GarryNet.Net.Result, function()
	-- TODO: read success + message, show them in the menu (and/or notification.AddLegacy).
end)

-- Requests to the server. The server re-checks superadmin on every one.
function GarryNet.SendLink(access_key)
	-- TODO: net.Start(GarryNet.Net.Link); net.WriteString(access_key); net.SendToServer()
end

function GarryNet.SendUnlink()
	-- TODO: net.Start(GarryNet.Net.Unlink); net.SendToServer()
end

function GarryNet.SendForceSync()
	-- TODO: net.Start(GarryNet.Net.ForceSync); net.SendToServer()
end
