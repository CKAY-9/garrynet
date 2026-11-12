-- The !gstore menu (docs/ADDON.md "!gstore"). Any player can see their recent purchases on this
-- server and get a link to its store:
--   GET /gnet/server/{SERVER_ID}/store?steam_id=   -> { store = { id, name, url } | null, purchases }
--
-- Nothing is cached: each opening asks GarryNet, so a purchase shows up as soon as it's paid.
-- Players can open it once every Config.StoreMenuCooldown seconds.

GarryNet.StoreMenu = GarryNet.StoreMenu or {}
local M = GarryNet.StoreMenu

local NO_STORE = "This server doesn't have a store set up."

-- Players whose request is in flight, by SteamID64.
local Loading = {}

local RECEIPT_STATUSES = { PAID = true, REFUNDED = true, DISPUTED = true }
local DELIVERY_STATUSES = { PENDING = true, FULFILLED = true, FAILED = true, REVOKED = true }

-- A purchase from GarryNet's response, or nil if it isn't one.
local function ParsePurchase(p)
	if not istable(p) then return nil end
	local price, purchased_at = tonumber(p.price), tonumber(p.purchased_at)
	if not isstring(p.item_name) or not isstring(p.currency) or not price or not purchased_at
		or not RECEIPT_STATUSES[p.status] or not DELIVERY_STATUSES[p.delivery] then
		return nil
	end
	return {
		item_name = p.item_name,
		price = price,
		currency = p.currency,
		status = p.status,
		delivery = p.delivery,
		purchased_at = purchased_at,
	}
end

-- Fetch the player's store menu from GarryNet and open it on their client. Explains in chat
-- when there's no store, or it couldn't be loaded.
function M.Open(ply)
	if not IsValid(ply) or ply:IsBot() then return end
	if not GarryNet.Storage.IsLinked() or not GarryNet.HTTP.IsConfigured() then
		GarryNet.NotifyPlayer(ply, NO_STORE)
		return
	end

	local steam_id = ply:SteamID64()
	if Loading[steam_id] then return end
	if (ply.GarryNetNextStoreMenu or 0) > CurTime() then
		GarryNet.NotifyPlayer(ply, "Wait a few seconds before opening the store again.")
		return
	end
	ply.GarryNetNextStoreMenu = CurTime() + GarryNet.Config.StoreMenuCooldown
	Loading[steam_id] = true

	GarryNet.HTTP.ServerRequest("GET", "/store?steam_id=" .. steam_id, nil, function(ok, status, data, err, current)
		Loading[steam_id] = nil
		if current and status == 401 then GarryNet.Link.OnOutOfSync() end
		if not IsValid(ply) then return end

		if not ok or not current or not istable(data) then
			GarryNet.Util.Warn("Couldn't load the store menu for " .. steam_id .. ": "
				.. (err or "unexpected response from GarryNet"))
			GarryNet.NotifyPlayer(ply, "The store couldn't be loaded right now. Try again in a minute.", false)
			return
		end

		local store = data.store
		if not istable(store) or not isstring(store.name) or not isstring(store.url) then
			GarryNet.NotifyPlayer(ply, NO_STORE)
			return
		end

		local purchases = {}
		for _, p in ipairs(istable(data.purchases) and data.purchases or {}) do
			purchases[#purchases + 1] = ParsePurchase(p)
		end

		net.Start(GarryNet.Net.StoreMenu)
		GarryNet.StoreMenuData.Write({ name = store.name, url = store.url, purchases = purchases })
		net.Send(ply)
	end)
end
