-- The !gnet menu: dark-blue panel over a darkened, blurred background (docs/ADDON.md).
--
-- Not linked: text box for the server access key (+ "Link" button).
-- Linked:     link details, last sync, and buttons for "Sync now" and "Unlink".

GarryNet.Menu = GarryNet.Menu or {}

local UI = GarryNet.UI
local Colors = UI.Colors
GarryNet.Menu.Colors = Colors -- cl_net.lua's chat notices use these

local WIDTH = 480
local PAD = 24
local GAP = 12

local Overlay = nil -- full-screen root (blur + darkening)
local Card = nil -- the dark-blue panel
local View = nil -- "linked" | "unlinked", what Card currently shows
local Refs = {} -- widgets the current view updates in place
local Result = nil -- { success, message }: outcome of the last action, shown under the buttons
local PendingUntil = 0 -- RealTime() until which a sent request counts as in flight
local ConfirmUnlinkUntil = 0 -- RealTime() until which a second Unlink click confirms
local SaveWaitUntil = 0 -- RealTime() until which saved store chat message settings count as in flight
local SavePending = false -- the next store chat message settings from the server are what we saved

local function IsBusy()
	return GarryNet.ClientState.busy or RealTime() < PendingUntil
end

-- Rows are laid out top to bottom; a row whose height is 0 is skipped.
local Rows = {}

local function AddRow(panel, height, gap)
	Rows[#Rows + 1] = { panel = panel, height = height, gap = gap or GAP }
	return panel
end

local function Layout()
	if not IsValid(Card) then return end
	local inner = WIDTH - PAD * 2
	local y = PAD
	local last_gap = 0

	for _, row in ipairs(Rows) do
		local h = isfunction(row.height) and row.height(inner) or row.height
		row.panel:SetVisible(h > 0)
		if h > 0 then
			row.panel:SetPos(PAD, y)
			row.panel:SetSize(inner, h)
			y = y + h + row.gap
			last_gap = row.gap
		end
	end

	Card:SetSize(WIDTH, y - last_gap + PAD)
	Card:Center()
end

-- A block of wrapped text. Set .text/.color with SetContent, which re-lays out the menu if
-- the height changed.
local function TextBlock(parent, font, color, text)
	local block = vgui.Create("DPanel", parent)
	block.text, block.font, block.color = text or "", font, color

	function block:WantedHeight(width)
		if self.text == "" then return 0 end
		surface.SetFont(self.font)
		local _, line_h = surface.GetTextSize("Ay")
		self.lines = UI.WrapLines(self.text, self.font, width)
		self.line_h = line_h
		return #self.lines * line_h
	end

	function block:SetContent(new_text, new_color)
		new_color = new_color or self.color
		if new_text == self.text and new_color == self.color then return end
		local before = self:WantedHeight(WIDTH - PAD * 2)
		self.text, self.color = new_text, new_color
		if self:WantedHeight(WIDTH - PAD * 2) ~= before then Layout() end
	end

	function block:Paint()
		for i, line in ipairs(self.lines or {}) do
			draw.SimpleText(line, self.font, 0, (i - 1) * self.line_h, self.color)
		end
		return true
	end

	AddRow(block, function(width) return block:WantedHeight(width) end)
	return block
end

local Button = UI.Button

-- A row of buttons sharing the width equally.
local function ButtonRow(parent, buttons)
	local row = vgui.Create("DPanel", parent)
	row:SetPaintBackground(false)
	for _, b in ipairs(buttons) do b:SetParent(row) end

	function row:PerformLayout(w, h)
		local count = #buttons
		local bw = (w - (count - 1) * 10) / count
		for i, b in ipairs(buttons) do
			b:SetPos((i - 1) * (bw + 10), 0)
			b:SetSize(bw, h)
		end
	end

	AddRow(row, 38)
	return row
end

local function Header(parent)
	AddRow(UI.Header(parent, "Server link", function() GarryNet.Menu.Close() end), 52, 16)
end

local function ResultBlock(parent)
	local block = TextBlock(parent, "GarryNet.Small", Colors.Muted, "")
	Refs.result = block
	return block
end

local function UpdateResult()
	if not IsValid(Refs.result) then return end
	if Result then
		Refs.result:SetContent(Result.message, Result.success and Colors.Success or Colors.Error)
	else
		Refs.result:SetContent("")
	end
end

local function NotConfiguredWarning(parent)
	local state = GarryNet.ClientState
	local text = ""
	if not state.configured then
		text = "garrynet_api_url is not set on this server. Add it to server.cfg before linking."
	elseif state.insecure then
		text = "garrynet_api_url doesn't use https://, so the access key and server token are sent unencrypted. "
			.. "Use an https:// URL unless GarryNet is on a private network you fully trust."
	end
	TextBlock(parent, "GarryNet.Small", Colors.Warning, text)
end

local function BuildUnlinked(card)
	TextBlock(card, "GarryNet.Body", Colors.Text, "This server isn't linked to GarryNet.")
	TextBlock(card, "GarryNet.Small", Colors.Muted,
		"Register the server on the website (Admin > Servers), then paste the access key it gives you here. Keys only work once.")
	NotConfiguredWarning(card)

	local entry = UI.TextEntry(card, "GarryNet.Body", "gnet_...")
	AddRow(entry, 38)
	Refs.entry = entry

	local function Submit()
		if IsBusy() then return end
		local key = string.Trim(entry:GetValue())
		if not GarryNet.Util.IsValidAccessKeyFormat(key) then
			Result = { success = false, message = "That isn't an access key. Keys look like gnet_ followed by 64 hex characters." }
			UpdateResult()
			return
		end
		PendingUntil = RealTime() + 20
		Result = nil
		UpdateResult()
		GarryNet.SendLink(key)
	end
	entry.OnEnter = Submit

	local link = Button(card, "Link server", Colors.Accent, Submit)
	ButtonRow(card, { link })
	Refs.link = link

	ResultBlock(card)

	return function()
		local busy = IsBusy()
		link:SetEnabled(not busy)
		link:SetText(busy and "Linking..." or "Link server")
	end
end

local function FormatAgo(seconds)
	seconds = math.max(0, math.floor(seconds))
	if seconds < 60 then return seconds .. "s ago" end
	if seconds < 3600 then return math.floor(seconds / 60) .. "m ago" end
	return math.floor(seconds / 3600) .. "h ago"
end

local function FormatIn(seconds)
	seconds = math.max(0, math.ceil(seconds))
	if seconds < 60 then return seconds .. "s" end
	if seconds < 3600 then return math.ceil(seconds / 60) .. "m" end
	return math.floor(seconds / 3600) .. "h " .. math.floor(seconds % 3600 / 60) .. "m"
end

local function Divider(parent)
	local line = vgui.Create("DPanel", parent)
	function line:Paint(w, h)
		surface.SetDrawColor(Colors.Border)
		surface.DrawRect(0, 0, w, h)
		return true
	end
	AddRow(line, 1, 16)
end

-- Repeating store chat messages: a switch, the interval, the message and Save. Nothing changes
-- on the server until Save. Returns the view's update function.
local function StoreMessagesForm(card)
	local cfg = GarryNet.Config.StoreMessages

	local title = vgui.Create("DPanel", card)
	local switch = UI.Switch(title)
	function title:Paint(w, h)
		draw.SimpleText("Store chat messages", "GarryNet.Body", 0, h / 2, Colors.Text, TEXT_ALIGN_LEFT, TEXT_ALIGN_CENTER)
		return true
	end
	function title:PerformLayout(w, h)
		switch:SetPos(w - switch:GetWide(), (h - switch:GetTall()) / 2)
	end
	AddRow(title, 24, 6)

	TextBlock(card, "GarryNet.Small", Colors.Muted,
		"Advertise the store to every player in chat. Only sent while this server has a store on GarryNet.")

	local interval = vgui.Create("DPanel", card)
	local minutes = UI.TextEntry(interval, "GarryNet.Body")
	minutes:SetNumeric(true)
	function interval:Paint(w, h)
		draw.SimpleText("Send every", "GarryNet.Small", 0, h / 2, Colors.Muted, TEXT_ALIGN_LEFT, TEXT_ALIGN_CENTER)
		draw.SimpleText("minutes", "GarryNet.Small", self.after_x or 0, h / 2, Colors.Muted, TEXT_ALIGN_LEFT, TEXT_ALIGN_CENTER)
		return true
	end
	function interval:PerformLayout(w, h)
		surface.SetFont("GarryNet.Small")
		local label_w = surface.GetTextSize("Send every")
		minutes:SetPos(label_w + 8, 0)
		minutes:SetSize(64, h)
		self.after_x = label_w + 8 + 64 + 8
	end
	AddRow(interval, 32)

	local message = UI.TextEntry(card, "GarryNet.Small")
	message:SetMultiline(true)
	-- Stop typing at the limit (pasting can still go over; Save then explains).
	function message:AllowInput()
		return (utf8.len(self:GetValue()) or 0) >= cfg.MaxLength
	end
	function message:PaintOver(w, h)
		local count = utf8.len(self:GetValue()) or 0
		draw.SimpleText(count .. "/" .. cfg.MaxLength, "GarryNet.Small", w - 6, h - 4,
			count > cfg.MaxLength and Colors.Error or Colors.Muted, TEXT_ALIGN_RIGHT, TEXT_ALIGN_BOTTOM)
	end
	AddRow(message, 64)

	local info = TextBlock(card, "GarryNet.Small", Colors.Muted, "")

	-- The settings as last saved on the server, and the form's values.
	local saved = nil
	local function Current()
		return { enabled = switch:GetChecked(), minutes = tonumber(minutes:GetValue()), message = message:GetValue() }
	end
	local function Differs(a, b)
		return a.enabled ~= b.enabled or a.minutes ~= b.minutes or a.message ~= b.message
	end
	local function Fill(s)
		switch:SetChecked(s.enabled)
		minutes:SetValue(tostring(s.minutes))
		message:SetValue(s.message)
	end

	local save = Button(card, "Save", Colors.Accent, function()
		if RealTime() < SaveWaitUntil then return end
		local c = Current()
		if not c.minutes or c.minutes ~= math.floor(c.minutes) or c.minutes < cfg.MinMinutes or c.minutes > cfg.MaxMinutes then
			Result = { success = false, message = string.format("The interval must be a whole number of minutes, %d to %d.",
				cfg.MinMinutes, cfg.MaxMinutes) }
			UpdateResult()
			return
		end
		if string.Trim(c.message) == "" then
			Result = { success = false, message = "The message can't be empty." }
			UpdateResult()
			return
		end
		SaveWaitUntil = RealTime() + 10
		SavePending = true
		Result = nil
		UpdateResult()
		GarryNet.SendStoreMessages(c.enabled, c.minutes, c.message)
	end)
	ButtonRow(card, { save })

	return function()
		local s = GarryNet.ClientState.store_messages
		if not s then return end

		if not saved then
			Fill(s)
		elseif s ~= saved then
			-- A new status. After our save is confirmed (the result comes first), it holds what
			-- we saved, possibly tidied by the server. Otherwise another superadmin's change
			-- replaces the form unless it has unsaved edits.
			local ours = SavePending and RealTime() >= SaveWaitUntil
			if ours or (Differs(s, saved) and not Differs(Current(), saved)) then Fill(s) end
			if ours then SavePending = false end
		end
		saved = s

		local dirty = Differs(Current(), saved)
		local waiting = RealTime() < SaveWaitUntil
		save:SetEnabled(dirty and not waiting)
		save:SetText(waiting and "Saving..." or (dirty and "Save" or "Saved"))

		if dirty then
			info:SetContent("You have unsaved changes.", Colors.Warning)
		elseif not s.enabled then
			info:SetContent("Off.", Colors.Muted)
		elseif s.skipped ~= "" then
			info:SetContent("The last message wasn't sent: " .. s.skipped, Colors.Warning)
		elseif s.next_at then
			info:SetContent("Next message in " .. FormatIn(s.next_at - RealTime()) .. ".", Colors.Muted)
		else
			info:SetContent("")
		end
	end
end

local function BuildLinked(card)
	local linked_as = TextBlock(card, "GarryNet.Body", Colors.Text, "")
	local sync = TextBlock(card, "GarryNet.Small", Colors.Muted, "")
	NotConfiguredWarning(card)

	local sync_button = Button(card, "Sync now", Colors.Accent, function()
		if IsBusy() then return end
		PendingUntil = RealTime() + 20
		GarryNet.SendForceSync()
	end)

	local unlink = Button(card, "Unlink", Colors.Danger, function()
		if IsBusy() then return end
		if RealTime() > ConfirmUnlinkUntil then
			ConfirmUnlinkUntil = RealTime() + 4
			return
		end
		ConfirmUnlinkUntil = 0
		PendingUntil = RealTime() + 20
		GarryNet.SendUnlink()
	end)

	ButtonRow(card, { sync_button, unlink })

	Divider(card)
	local update_messages = StoreMessagesForm(card)

	-- Under everything, so the outcome of any button shows in the same place.
	ResultBlock(card)

	return function()
		update_messages()

		local state = GarryNet.ClientState
		linked_as:SetContent(string.format("Linked as \"%s\" (#%d)", state.server_name or "", state.server_id or 0))

		local last = state.last_sync
		if not last then
			sync:SetContent("Waiting for the first sync...", Colors.Muted)
		elseif last.ok then
			sync:SetContent("Last sync " .. FormatAgo(RealTime() - last.at) .. ": " .. last.message, Colors.Muted)
		else
			sync:SetContent("Last sync failed " .. FormatAgo(RealTime() - last.at) .. ": " .. last.message, Colors.Error)
		end

		local busy = IsBusy()
		sync_button:SetEnabled(not busy)
		unlink:SetEnabled(not busy)
		unlink:SetText(RealTime() < ConfirmUnlinkUntil and "Click again to unlink" or "Unlink")
	end
end

-- (Re)build the card's content for the current state.
local function Build()
	if IsValid(Card) then Card:Remove() end
	Rows, Refs = {}, {}

	Card = UI.Card(Overlay)
	Header(Card)
	View = GarryNet.ClientState.linked and "linked" or "unlinked"
	Refs.update = View == "linked" and BuildLinked(Card) or BuildUnlinked(Card)
	Refs.update()
	UpdateResult()
	Layout()

	-- Keep "Xs ago", busy and confirm states current.
	local next_update = 0
	function Card:Think()
		if RealTime() < next_update then return end
		next_update = RealTime() + 0.25
		if Refs.update then Refs.update() end
	end
end

function GarryNet.Menu.IsOpen()
	return IsValid(Overlay)
end

function GarryNet.Menu.Close()
	if IsValid(Overlay) then Overlay:Remove() end
	Overlay, Card, View = nil, nil, nil
	Rows, Refs = {}, {}
	Result = nil
end

-- Build and show the menu for the current GarryNet.ClientState.
function GarryNet.Menu.Open()
	if GarryNet.Menu.IsOpen() then
		GarryNet.Menu.Refresh()
		return
	end

	-- One GarryNet menu at a time.
	if GarryNet.StoreMenu then GarryNet.StoreMenu.Close() end

	Overlay = UI.Overlay(GarryNet.Menu.Close)
	Build()
end

-- Rebuild the content after a status change.
function GarryNet.Menu.Refresh()
	if not GarryNet.Menu.IsOpen() then return end
	-- A status update means the server finished (or started) what we asked for.
	if not GarryNet.ClientState.busy then PendingUntil = 0 end

	local view = GarryNet.ClientState.linked and "linked" or "unlinked"
	if view ~= View then
		Build()
	elseif Refs.update then
		Refs.update()
	end
end

-- Show an action result (success/failure message) inside the menu.
function GarryNet.Menu.ShowResult(success, message)
	PendingUntil = 0
	SaveWaitUntil = 0
	if not success then SavePending = false end
	Result = { success = success, message = message }
	UpdateResult()
	if Refs.update then Refs.update() end
end
