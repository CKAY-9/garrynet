-- Client side of the net messages. Keeps the latest status for the menu to render.

GarryNet.ClientState = GarryNet.ClientState or {
	linked = false,
	configured = true,
	busy = false,
}

local function SetState(status)
	-- last_sync.ago is relative to when the server sent it; anchor it to our clock.
	if status.last_sync then status.last_sync.at = RealTime() - status.last_sync.ago end
	GarryNet.ClientState = status
end

local function ChatNotice(message, color)
	chat.AddText(GarryNet.Menu.Colors.Accent, "[GarryNet] ", color or GarryNet.Menu.Colors.Text, message)
end

net.Receive(GarryNet.Net.OpenMenu, function()
	SetState(GarryNet.Status.Read())
	GarryNet.Menu.Open()
end)

net.Receive(GarryNet.Net.Status, function()
	SetState(GarryNet.Status.Read())
	if GarryNet.Menu.IsOpen() then GarryNet.Menu.Refresh() end
end)

net.Receive(GarryNet.Net.Result, function()
	local success = net.ReadBool()
	local message = net.ReadString()

	if GarryNet.Menu.IsOpen() then
		GarryNet.Menu.ShowResult(success, message)
	else
		ChatNotice(message, success and GarryNet.Menu.Colors.Success or GarryNet.Menu.Colors.Error)
	end
end)

net.Receive(GarryNet.Net.Notify, function()
	local message = net.ReadString()
	local has_outcome = net.ReadBool()
	local success = net.ReadBool()

	local color = nil
	if has_outcome then color = success and GarryNet.Menu.Colors.Success or GarryNet.Menu.Colors.Error end
	ChatNotice(message, color)
end)

-- Requests to the server. The server re-checks superadmin on every one.
function GarryNet.SendLink(access_key)
	net.Start(GarryNet.Net.Link)
	net.WriteString(access_key)
	net.SendToServer()
end

function GarryNet.SendUnlink()
	net.Start(GarryNet.Net.Unlink)
	net.SendToServer()
end

function GarryNet.SendForceSync()
	net.Start(GarryNet.Net.ForceSync)
	net.SendToServer()
end
