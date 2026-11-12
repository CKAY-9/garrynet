-- The look shared by the GarryNet menus (!gnet, !gstore): a dark-blue card with an accent strip,
-- over a darkened, blurred background (docs/ADDON.md).

GarryNet.UI = GarryNet.UI or {}
local UI = GarryNet.UI

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
UI.Colors = Colors

surface.CreateFont("GarryNet.Title", { font = "Roboto", size = 28, weight = 700, extended = true })
surface.CreateFont("GarryNet.Body", { font = "Roboto", size = 18, weight = 500, extended = true })
surface.CreateFont("GarryNet.Small", { font = "Roboto", size = 15, weight = 500, extended = true })
surface.CreateFont("GarryNet.Button", { font = "Roboto", size = 17, weight = 700, extended = true })

function UI.Lighten(c, amount)
	return Color(math.min(c.r + amount, 255), math.min(c.g + amount, 255), math.min(c.b + amount, 255), c.a)
end

-- Split `text` into lines no wider than `width` in `font`.
function UI.WrapLines(text, font, width)
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

-- `text` cut short with "..." so it fits in `width` in `font`.
function UI.Ellipsize(text, font, width)
	surface.SetFont(font)
	if surface.GetTextSize(text) <= width then return text end
	local chars = {}
	for _, code in utf8.codes(text) do chars[#chars + 1] = utf8.char(code) end
	local lo, hi = 0, #chars
	while lo < hi do
		local mid = math.ceil((lo + hi) / 2)
		if surface.GetTextSize(table.concat(chars, "", 1, mid) .. "...") <= width then
			lo = mid
		else
			hi = mid - 1
		end
	end
	return table.concat(chars, "", 1, lo) .. "..."
end

-- A filled, rounded button that lightens on hover and fades when disabled.
function UI.Button(parent, label, color, on_click)
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
			c = UI.Lighten(c, 25)
		end
		draw.RoundedBox(6, 0, 0, w, h, c)
	end

	return button
end

-- A rounded text box that lights up in the accent color while focused. `placeholder` shows
-- while it's empty.
function UI.TextEntry(parent, font, placeholder)
	local entry = vgui.Create("DTextEntry", parent)
	entry:SetFont(font)
	entry.placeholder = placeholder or ""

	function entry:Paint(w, h)
		draw.RoundedBox(6, 0, 0, w, h, self:HasFocus() and Colors.Accent or Colors.Border)
		draw.RoundedBox(6, 1, 1, w - 2, h - 2, Colors.Input)
		if self:GetValue() == "" and self.placeholder ~= "" then
			draw.SimpleText(self.placeholder, font, 5, h / 2, Colors.Muted, TEXT_ALIGN_LEFT, TEXT_ALIGN_CENTER)
		end
		self:DrawTextEntryText(Colors.Text, Colors.Accent, Colors.Text)
	end

	return entry
end

-- An on/off switch (40x22). `on_change(checked)` runs when it's clicked.
function UI.Switch(parent, on_change)
	local switch = vgui.Create("DButton", parent)
	switch:SetText("")
	switch:SetSize(40, 22)
	switch.checked = false
	switch.knob = 0 -- 0 (off) to 1 (on), eased towards `checked`

	function switch:SetChecked(checked) self.checked = checked == true end
	function switch:GetChecked() return self.checked end

	function switch:DoClick()
		self.checked = not self.checked
		if on_change then on_change(self.checked) end
	end

	function switch:Paint(w, h)
		self.knob = Lerp(math.min(FrameTime() * 15, 1), self.knob, self.checked and 1 or 0)
		local track = self.checked and Colors.Accent or Colors.Neutral
		if self:IsHovered() then track = UI.Lighten(track, 20) end
		draw.RoundedBox(h / 2, 0, 0, w, h, track)
		local size = h - 6
		draw.RoundedBox(size / 2, 3 + (w - size - 6) * self.knob, 3, size, size, Colors.Text)
		return true
	end

	return switch
end

-- The X in a card's top-right corner (24x24).
function UI.CloseButton(parent, on_click)
	local close = vgui.Create("DButton", parent)
	close:SetText("")
	close:SetSize(24, 24)
	close.DoClick = on_click
	function close:Paint(w, h)
		surface.SetDrawColor(self:IsHovered() and Colors.Text or Colors.Muted)
		surface.DrawLine(6, 6, w - 6, h - 6)
		surface.DrawLine(w - 6, 6, 6, h - 6)
	end
	return close
end

-- The full-screen root of a menu: blurs and darkens the game. Escape or a click outside the
-- card calls `on_close`, which should remove it.
function UI.Overlay(on_close)
	local overlay = vgui.Create("EditablePanel")
	overlay:SetSize(ScrW(), ScrH())
	overlay:MakePopup()
	overlay:SetKeyboardInputEnabled(true)
	overlay.start_time = SysTime()

	function overlay:Paint(w, h)
		Derma_DrawBackgroundBlur(self, self.start_time)
		surface.SetDrawColor(Colors.Overlay)
		surface.DrawRect(0, 0, w, h)
	end

	-- Escape closes the menu instead of (only) opening the game menu.
	function overlay:Think()
		if input.IsKeyDown(KEY_ESCAPE) then
			if gui.IsGameUIVisible() then gui.HideGameUI() end
			on_close()
		end
	end

	-- Clicking outside the card closes the menu.
	function overlay:OnMousePressed()
		on_close()
	end

	return overlay
end

-- The dark-blue card a menu's content sits on.
function UI.Card(parent)
	local card = vgui.Create("DPanel", parent)
	function card:Paint(w, h)
		-- Accent strip along the top: the accent shows above the background's top edge.
		draw.RoundedBox(10, 0, 0, w, h, Colors.Accent)
		draw.RoundedBoxEx(10, 0, 4, w, h - 4, Colors.Background, false, false, true, true)
		return true
	end
	-- Swallow clicks so only clicks outside the card close the menu.
	card:SetMouseInputEnabled(true)
	function card:OnMousePressed() end
	return card
end

-- "GarryNet" over a muted subtitle, with the close button on the right. 52px tall.
function UI.Header(parent, subtitle, on_close)
	local header = vgui.Create("DPanel", parent)
	header.subtitle = subtitle

	function header:Paint()
		draw.SimpleText("GarryNet", "GarryNet.Title", 0, 0, Colors.Text)
		draw.SimpleText(self.subtitle, "GarryNet.Small", 0, 32, Colors.Muted)
		return true
	end

	local close = UI.CloseButton(header, on_close)
	function header:PerformLayout(w)
		close:SetPos(w - 24, 4)
	end

	return header
end
