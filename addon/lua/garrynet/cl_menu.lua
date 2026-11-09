-- The !gnet menu: dark-blue panel over a darkened, blurred background (docs/ADDON.md).
--
-- Not linked: text box for the server access key (+ "Link" button).
-- Linked:     link details, last sync, and buttons for "Sync now" and "Unlink".

GarryNet.Menu = GarryNet.Menu or {}

local Colors = {
	Background = Color(12, 22, 48), -- dark blue panel
	Overlay = Color(0, 0, 0, 150), -- darkening behind the panel
	Input = Color(20, 34, 70),
	Border = Color(44, 66, 120),
	Accent = Color(59, 130, 246),
	Danger = Color(200, 55, 65),
	Neutral = Color(40, 58, 100),
	Text = Color(235, 240, 255),
	Muted = Color(150, 165, 200),
	Success = Color(74, 222, 128),
	Error = Color(248, 113, 113),
	Warning = Color(250, 204, 21),
}
GarryNet.Menu.Colors = Colors

surface.CreateFont("GarryNet.Title", { font = "Roboto", size = 28, weight = 700, extended = true })
surface.CreateFont("GarryNet.Body", { font = "Roboto", size = 18, weight = 500, extended = true })
surface.CreateFont("GarryNet.Small", { font = "Roboto", size = 15, weight = 500, extended = true })
surface.CreateFont("GarryNet.Button", { font = "Roboto", size = 17, weight = 700, extended = true })

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

local function IsBusy()
	return GarryNet.ClientState.busy or RealTime() < PendingUntil
end

local function Lighten(c, amount)
	return Color(math.min(c.r + amount, 255), math.min(c.g + amount, 255), math.min(c.b + amount, 255), c.a)
end

-- Split `text` into lines no wider than `width` in `font`.
local function WrapLines(text, font, width)
	surface.SetFont(font)
	local lines = {}
	for paragraph in (text .. "\n"):gmatch("(.-)\n") do
		local line = ""
		for word in paragraph:gmatch("%S+") do
			local candidate = line == "" and word or (line .. " " .. word)
			if line == "" or surface.GetTextSize(candidate) <= width then
				line = candidate
			else
				lines[#lines + 1] = line
				line = word
			end
		end
		lines[#lines + 1] = line
	end
	return lines
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
		self.lines = WrapLines(self.text, self.font, width)
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

local function Button(parent, label, color, on_click)
	local button = vgui.Create("DButton", parent)
	button:SetText(label)
	button:SetFont("GarryNet.Button")
	button:SetTextColor(Colors.Text)
	button.fill = color
	button.DoClick = on_click

	function button:Paint(w, h)
		local c = self.fill
		if not self:IsEnabled() then
			c = ColorAlpha(c, 90)
		elseif self:IsHovered() then
			c = Lighten(c, 25)
		end
		draw.RoundedBox(6, 0, 0, w, h, c)
	end

	return button
end

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
	local header = vgui.Create("DPanel", parent)

	function header:Paint(w, h)
		draw.SimpleText("GarryNet", "GarryNet.Title", 0, 0, Colors.Text)
		draw.SimpleText("Server link", "GarryNet.Small", 0, 32, Colors.Muted)
		return true
	end

	local close = vgui.Create("DButton", header)
	close:SetText("")
	close:SetSize(24, 24)
	close.DoClick = function() GarryNet.Menu.Close() end
	function close:Paint(w, h)
		local c = self:IsHovered() and Colors.Text or Colors.Muted
		surface.SetDrawColor(c)
		surface.DrawLine(6, 6, w - 6, h - 6)
		surface.DrawLine(w - 6, 6, 6, h - 6)
	end
	function header:PerformLayout(w)
		close:SetPos(w - 24, 4)
	end

	AddRow(header, 52, 16)
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
	TextBlock(parent, "GarryNet.Small", Colors.Warning, GarryNet.ClientState.configured and ""
		or "garrynet_api_url is not set on this server. Add it to server.cfg before linking.")
end

local function BuildUnlinked(card)
	TextBlock(card, "GarryNet.Body", Colors.Text, "This server isn't linked to GarryNet.")
	TextBlock(card, "GarryNet.Small", Colors.Muted,
		"Register the server on the website (Admin > Servers), then paste the access key it gives you here. Keys only work once.")
	NotConfiguredWarning(card)

	local entry = vgui.Create("DTextEntry", card)
	entry:SetFont("GarryNet.Body")
	function entry:Paint(w, h)
		draw.RoundedBox(6, 0, 0, w, h, self:HasFocus() and Colors.Accent or Colors.Border)
		draw.RoundedBox(6, 1, 1, w - 2, h - 2, Colors.Input)
		if self:GetValue() == "" then
			draw.SimpleText("gnet_...", "GarryNet.Body", 5, h / 2, Colors.Muted, TEXT_ALIGN_LEFT, TEXT_ALIGN_CENTER)
		end
		self:DrawTextEntryText(Colors.Text, Colors.Accent, Colors.Text)
	end
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
	ResultBlock(card)

	return function()
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

	Card = vgui.Create("DPanel", Overlay)
	function Card:Paint(w, h)
		-- Accent strip along the top: the accent shows above the background's top edge.
		draw.RoundedBox(10, 0, 0, w, h, Colors.Accent)
		draw.RoundedBoxEx(10, 0, 4, w, h - 4, Colors.Background, false, false, true, true)
		return true
	end
	-- Swallow clicks so only clicks outside the card close the menu.
	Card:SetMouseInputEnabled(true)
	function Card:OnMousePressed() end

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

	Overlay = vgui.Create("EditablePanel")
	Overlay:SetSize(ScrW(), ScrH())
	Overlay:MakePopup()
	Overlay:SetKeyboardInputEnabled(true)
	Overlay.start_time = SysTime()

	function Overlay:Paint(w, h)
		Derma_DrawBackgroundBlur(self, self.start_time)
		surface.SetDrawColor(Colors.Overlay)
		surface.DrawRect(0, 0, w, h)
	end

	-- Escape closes the menu instead of (only) opening the game menu.
	function Overlay:Think()
		if input.IsKeyDown(KEY_ESCAPE) then
			if gui.IsGameUIVisible() then gui.HideGameUI() end
			GarryNet.Menu.Close()
		end
	end

	-- Clicking outside the card closes the menu.
	function Overlay:OnMousePressed()
		GarryNet.Menu.Close()
	end

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
	Result = { success = success, message = message }
	UpdateResult()
	if Refs.update then Refs.update() end
end
