-- Shared constants. Nothing here talks to the backend.

GarryNet.Config = {
	-- Seconds between interval syncs (docs/ADDON.md: every minute).
	IntervalSeconds = 60,

	-- HTTP request timeout in seconds.
	HttpTimeout = 15,

	-- Chat command that opens the menu (superadmins only).
	ChatCommand = "!gnet",

	-- Chat command that opens the store menu: the player's recent purchases and a link to the
	-- server's store (anyone).
	StoreCommand = "!gstore",

	-- Seconds a player waits between opening the store menu (each one asks GarryNet).
	StoreMenuCooldown = 5,

	-- Directory (inside garrysmod/data) where link state is persisted.
	DataDir = "garrynet",

	-- Server access keys look like "gnet_" followed by 64 hex characters (docs/SERVERS.md).
	AccessKeyPattern = "^gnet_%x+$",
	AccessKeyLength = 69,

	-- Repeating chat messages advertising the store, set from the !gnet menu and saved in
	-- garrysmod/data/garrynet/store_messages.json. These are the limits and the defaults.
	StoreMessages = {
		DefaultMinutes = 10,
		MinMinutes = 1,
		MaxMinutes = 1440,
		MaxLength = 200, -- characters
		DefaultMessage = "Support the server and get in-game rewards: type !gstore to visit the store.",
	},

	-- Store delivery (docs/STORE.md). Server-side only.
	Store = {
		-- Played to the buyer when a purchase is delivered (a path under garrysmod/sound; "" for none).
		RewardSound = "garrysmod/content_downloaded.wav",

		-- Seconds between fetches of the delivery queue.
		PollSeconds = 60,

		-- Seconds after a player joins before their purchases are delivered, so the gamemode
		-- (e.g. DarkRP's money) and the admin mod have loaded them.
		JoinDelay = 10,

		-- Seconds between checks for expired timed usergroups.
		ReconcileSeconds = 60,

		-- Where players go when their store usergroup ends.
		DefaultGroup = "user",

		-- Usergroups a store usergroup may replace. Anyone in another group (staff) keeps it.
		OverridableGroups = { "user" },

		-- Never granted by the store, whatever GarryNet says (lowercase).
		ProtectedGroups = { "superadmin", "admin", "owner", "developer" },

		-- Longest wait between retries of a purchase that failed to deliver.
		MaxRetryMinutes = 60,
	},
}

-- Backend base URL: wherever the GarryNet backend is reachable, e.g. "https://api.example.com".
-- Set in server.cfg: garrynet_api_url "https://..."
if SERVER then
	CreateConVar("garrynet_api_url", "", FCVAR_PROTECTED + FCVAR_ARCHIVE, "Base URL of the GarryNet backend.")
end

-- Net message names, shared so both realms agree on them.
GarryNet.Net = {
	OpenMenu = "GarryNet.OpenMenu", -- server -> client: open the menu, payload is the current status
	Status = "GarryNet.Status", -- server -> client: link status update
	Link = "GarryNet.Link", -- client -> server: access key submitted
	Unlink = "GarryNet.Unlink", -- client -> server: unlink requested
	ForceSync = "GarryNet.ForceSync", -- client -> server: force an interval update
	Result = "GarryNet.Result", -- server -> client: result of Link/Unlink/ForceSync (success, message)
	RewardSound = "GarryNet.RewardSound", -- server -> client: play the reward sound
	Notify = "GarryNet.Notify", -- server -> client: chat notice (superadmins: linked, out of sync; buyers: purchase delivered)
	StoreMenu = "GarryNet.StoreMenu", -- server -> client: open the store menu (sh_store_menu.lua)
	SaveStoreMessages = "GarryNet.SaveStoreMessages", -- client -> server: store chat message settings (enabled, minutes, message)
}
