-- The !gstore menu (docs/ADDON.md "!gstore"): the player's recent purchases on this server and a
-- button that opens the server's store on the website. Same look as the !gnet menu (cl_ui.lua).

GarryNet.StoreMenu = GarryNet.StoreMenu or {}

local UI = GarryNet.UI
local Colors = UI.Colors

local WIDTH = 520
local PAD = 24
local GAP = 12
local ROW_H = 52
local VISIBLE_ROWS = 6 -- more than this scrolls

local Overlay = nil

local SYMBOLS = { usd = "$", eur = "€", gbp = "£", jpy = "¥" }
local ZERO_DECIMAL = { jpy = true } -- prices are in the currency's minor unit, which for these is the unit itself

local function FormatPrice(minor, currency)
	currency = string.lower(currency)
	local amount = ZERO_DECIMAL[currency] and string.format("%d", minor) or string.format("%.2f", minor / 100)
	amount = string.Comma(amount)
	local symbol = SYMBOLS[currency]
	return symbol and (symbol .. amount) or (amount .. " " .. string.upper(currency))
end

local function FormatDate(unix)
	return os.date("%b ", unix) .. tonumber(os.date("%d", unix)) .. os.date(", %Y", unix)
end

-- What happened to a purchase, in a word or two, and its color.
local function Describe(p)
	if p.status == "REFUNDED" then return "Refunded", Colors.Muted end
	if p.status == "DISPUTED" then return "Disputed", Colors.Muted end
	if p.delivery == "FULFILLED" then return "Delivered", Colors.Success end
	if p.delivery == "FAILED" then return "Delivery failed", Colors.Error end
	if p.delivery == "REVOKED" then return "Revoked", Colors.Muted end
	return "Awaiting delivery", Colors.Warning
end

-- A line under the list when a purchase hasn't arrived, or "" if they all have.
local function Hint(purchases)
	local waiting = false
	for _, p in ipairs(purchases) do
		if p.status == "PAID" and p.delivery == "FAILED" then
			return "Failed deliveries are retried automatically. If one keeps failing, contact staff."
		end
		if p.status == "PAID" and p.delivery == "PENDING" then waiting = true end
	end
	return waiting and "Purchases are delivered while you're on this server, usually within a minute." or ""
end

local function PurchaseRow(parent, p, last)
	local row = vgui.Create("DPanel", parent)
	row:Dock(TOP)
	row:SetTall(ROW_H)
	local status, status_color = Describe(p)
	local price = FormatPrice(p.price, p.currency)
	local date = FormatDate(p.purchased_at)

	function row:Paint(w, h)
		local x, right = 12, w - 12
		surface.SetFont("GarryNet.Body")
		local price_w = surface.GetTextSize(price)
		local name = UI.Ellipsize(p.item_name, "GarryNet.Body", right - x - price_w - 16)

		draw.SimpleText(name, "GarryNet.Body", x, 8, Colors.Text)
		draw.SimpleText(price, "GarryNet.Body", right, 8, Colors.Text, TEXT_ALIGN_RIGHT)
		draw.SimpleText(date, "GarryNet.Small", x, 29, Colors.Muted)
		draw.SimpleText(status, "GarryNet.Small", right, 29, status_color, TEXT_ALIGN_RIGHT)

		if not last then
			surface.SetDrawColor(Colors.Border)
			surface.DrawRect(x, h - 1, w - x * 2, 1)
		end
		return true
	end

	return row
end

-- The bordered list of purchases; scrolls past VISIBLE_ROWS.
local function PurchaseList(parent, purchases)
	local box = vgui.Create("DPanel", parent)
	function box:Paint(w, h)
		draw.RoundedBox(6, 0, 0, w, h, Colors.Border)
		draw.RoundedBox(6, 1, 1, w - 2, h - 2, Colors.Input)
		if #purchases == 0 then
			draw.SimpleText("You haven't bought anything on this server yet.", "GarryNet.Small",
				w / 2, h / 2, Colors.Muted, TEXT_ALIGN_CENTER, TEXT_ALIGN_CENTER)
		end
		return true
	end

	if #purchases == 0 then return box, 64 end

	local scroll = vgui.Create("DScrollPanel", box)
	scroll:Dock(FILL)
	scroll:DockMargin(1, 1, 1, 1)
	local bar = scroll:GetVBar()
	bar:SetWide(6)
	bar:SetHideButtons(true)
	function bar:Paint() end
	function bar.btnGrip:Paint(w, h)
		draw.RoundedBox(3, 0, 0, w - 2, h, Colors.Border)
	end

	for i, p in ipairs(purchases) do
		PurchaseRow(scroll, p, i == #purchases)
	end

	return box, math.min(#purchases, VISIBLE_ROWS) * ROW_H + 2
end

function GarryNet.StoreMenu.IsOpen()
	return IsValid(Overlay)
end

function GarryNet.StoreMenu.Close()
	if IsValid(Overlay) then Overlay:Remove() end
	Overlay = nil
end

-- Show the menu for `data` (see sh_store_menu.lua).
function GarryNet.StoreMenu.Open(data)
	GarryNet.StoreMenu.Close()
	-- One GarryNet menu at a time.
	if GarryNet.Menu and GarryNet.Menu.IsOpen() then GarryNet.Menu.Close() end

	Overlay = UI.Overlay(GarryNet.StoreMenu.Close)
	local card = UI.Card(Overlay)
	local inner = WIDTH - PAD * 2
	local y = PAD

	local header = UI.Header(card, UI.Ellipsize(data.name, "GarryNet.Small", inner - 40), GarryNet.StoreMenu.Close)
	header:SetPos(PAD, y)
	header:SetSize(inner, 52)
	y = y + 52 + 16

	local label = vgui.Create("DLabel", card)
	label:SetFont("GarryNet.Small")
	label:SetTextColor(Colors.Muted)
	label:SetText("Your recent purchases")
	label:SetPos(PAD, y)
	label:SetSize(inner, 18)
	y = y + 18 + 6

	local list, list_h = PurchaseList(card, data.purchases)
	list:SetPos(PAD, y)
	list:SetSize(inner, list_h)
	y = y + list_h + GAP

	local hint = Hint(data.purchases)
	if hint ~= "" then
		local lines = UI.WrapLines(hint, "GarryNet.Small", inner)
		surface.SetFont("GarryNet.Small")
		local _, line_h = surface.GetTextSize("Ay")
		local block = vgui.Create("DPanel", card)
		function block:Paint()
			for i, line in ipairs(lines) do
				draw.SimpleText(line, "GarryNet.Small", 0, (i - 1) * line_h, Colors.Muted)
			end
			return true
		end
		block:SetPos(PAD, y)
		block:SetSize(inner, #lines * line_h)
		y = y + #lines * line_h + GAP
	end

	-- Only web links: the URL comes from GarryNet, but gui.OpenURL would take anything.
	local url = data.url
	local valid_url = string.match(url, "^https?://") ~= nil
	local open = UI.Button(card, "Open store", Colors.Accent, function()
		if not valid_url then return end
		gui.OpenURL(url)
		GarryNet.StoreMenu.Close()
	end)
	open:SetEnabled(valid_url)
	open:SetPos(PAD, y)
	open:SetSize(inner, 38)
	y = y + 38

	card:SetSize(WIDTH, y + PAD)
	card:Center()
end
