-- Client side of the net messages. Keeps the latest status for the menu to render.

GravNet.ClientState = GravNet.ClientState or {
	linked = false,
	configured = true,
	busy = false,
}

local function SetState(status)
	-- last_sync.ago is relative to when the server sent it; anchor it to our clock.
	if status.last_sync then status.last_sync.at = RealTime() - status.last_sync.ago end
	local messages = status.store_messages
	if messages and messages.next_in then messages.next_at = RealTime() + messages.next_in end
	GravNet.ClientState = status
end

local function ChatNotice(message, color)
	chat.AddText(GravNet.Menu.Colors.Accent, "[GravNet] ", color or GravNet.Menu.Colors.Text, message)
end

net.Receive(GravNet.Net.OpenMenu, function()
	SetState(GravNet.Status.Read())
	GravNet.Menu.Open()
end)

net.Receive(GravNet.Net.Status, function()
	SetState(GravNet.Status.Read())
	if GravNet.Menu.IsOpen() then GravNet.Menu.Refresh() end
end)

net.Receive(GravNet.Net.Result, function()
	local success = net.ReadBool()
	local message = net.ReadString()

	if GravNet.Menu.IsOpen() then
		GravNet.Menu.ShowResult(success, message)
	else
		ChatNotice(message, success and GravNet.Menu.Colors.Success or GravNet.Menu.Colors.Error)
	end
end)

net.Receive(GravNet.Net.Notify, function()
	local message = net.ReadString()
	local has_outcome = net.ReadBool()
	local success = net.ReadBool()

	local color = nil
	if has_outcome then color = success and GravNet.Menu.Colors.Success or GravNet.Menu.Colors.Error end
	ChatNotice(message, color)
end)

net.Receive(GravNet.Net.StoreMenu, function()
	GravNet.StoreMenu.Open(GravNet.StoreMenuData.Read())
end)

net.Receive(GravNet.Net.RewardSound, function()
	local path = GravNet.Config.Store.RewardSound
	if path ~= "" then surface.PlaySound(path) end
end)

-- Requests to the server. The server re-checks superadmin on every one.
function GravNet.SendLink(access_key)
	net.Start(GravNet.Net.Link)
	net.WriteString(access_key)
	net.SendToServer()
end

function GravNet.SendUnlink()
	net.Start(GravNet.Net.Unlink)
	net.SendToServer()
end

function GravNet.SendStoreMessages(enabled, minutes, message)
	net.Start(GravNet.Net.SaveStoreMessages)
	net.WriteBool(enabled)
	net.WriteUInt(minutes, 16)
	net.WriteString(message)
	net.SendToServer()
end

function GravNet.SendForceSync()
	net.Start(GravNet.Net.ForceSync)
	net.SendToServer()
end
