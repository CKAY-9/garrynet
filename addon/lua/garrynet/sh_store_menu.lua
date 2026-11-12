-- What the server sends a player for the !gstore menu. Shared so both realms read and write it
-- the same way.
--
-- data = {
--     name = string,             -- the store's name
--     url = string,              -- its page on the website
--     purchases = { {            -- newest first, at most MAX_PURCHASES
--         item_name = string,
--         price = number,        -- minor units of `currency`
--         currency = string,     -- ISO 4217, lowercase
--         status = string,       -- receipt: "PAID" | "REFUNDED" | "DISPUTED"
--         delivery = string,     -- fulfillment: "PENDING" | "FULFILLED" | "FAILED" | "REVOKED"
--         purchased_at = number, -- unix seconds
--     } },
-- }

GarryNet.StoreMenuData = GarryNet.StoreMenuData or {}
local D = GarryNet.StoreMenuData

D.MAX_PURCHASES = 20

function D.Write(data)
	net.WriteString(data.name)
	net.WriteString(data.url)
	local count = math.min(#data.purchases, D.MAX_PURCHASES)
	net.WriteUInt(count, 8)
	for i = 1, count do
		local p = data.purchases[i]
		net.WriteString(p.item_name)
		net.WriteDouble(p.price)
		net.WriteString(p.currency)
		net.WriteString(p.status)
		net.WriteString(p.delivery)
		net.WriteDouble(p.purchased_at)
	end
end

function D.Read()
	local data = { name = net.ReadString(), url = net.ReadString(), purchases = {} }
	for i = 1, net.ReadUInt(8) do
		data.purchases[i] = {
			item_name = net.ReadString(),
			price = net.ReadDouble(),
			currency = net.ReadString(),
			status = net.ReadString(),
			delivery = net.ReadString(),
			purchased_at = net.ReadDouble(),
		}
	end
	return data
end
