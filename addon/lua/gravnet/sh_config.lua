-- Shared constants. Nothing here talks to the backend.

GravNet.Config = {
	-- Seconds between interval syncs (docs/ADDON.md: every minute).
	IntervalSeconds = 60,

	-- HTTP request timeout in seconds.
	HttpTimeout = 15,

	-- Chat command that opens the menu (superadmins only).
	ChatCommand = "!gnet",

	-- Chat command that opens the store menu: the player's recent purchases and a link to the
	-- server's store (anyone).
	StoreCommand = "!gstore",

	-- Seconds a player waits between opening the store menu (each one asks GravNet).
	StoreMenuCooldown = 5,

	-- Directory (inside garrysmod/data) where link state is persisted.
	DataDir = "gravnet",

	-- Server access keys look like "gnet_" followed by 64 hex characters (docs/SERVERS.md).
	AccessKeyPattern = "^gnet_%x+$",
	AccessKeyLength = 69,

	-- Repeating chat messages advertising the store, set from the !gnet menu and saved in
	-- garrysmod/data/gravnet/store_messages.json. These are the limits and the defaults.
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

		-- Never granted by the store, whatever GravNet says (lowercase).
		ProtectedGroups = { "superadmin", "admin", "owner", "developer" },

		-- Longest wait between retries of a purchase that failed to deliver.
		MaxRetryMinutes = 60,
	},
}

-- Backend base URL: wherever the GravNet backend is reachable, e.g. "https://api.example.com".
-- Set in server.cfg: gravnet_api_url "https://..."
if SERVER then
	CreateConVar("gravnet_api_url", "", FCVAR_PROTECTED + FCVAR_ARCHIVE, "Base URL of the GravNet backend.")
	-- gravnet_api_url must be https:// unless GravNet runs on this machine. Development only:
	-- 1 allows plain HTTP to another machine (e.g. a backend on the LAN), with a warning.
	CreateConVar("gravnet_allow_http", "0", FCVAR_PROTECTED + FCVAR_ARCHIVE,
		"Development only: allow a plain-HTTP gravnet_api_url on another machine.", 0, 1)
end

-- Net message names, shared so both realms agree on them.
GravNet.Net = {
	OpenMenu = "GravNet.OpenMenu", -- server -> client: open the menu, payload is the current status
	Status = "GravNet.Status", -- server -> client: link status update
	Link = "GravNet.Link", -- client -> server: access key submitted
	Unlink = "GravNet.Unlink", -- client -> server: unlink requested
	ForceSync = "GravNet.ForceSync", -- client -> server: force an interval update
	Result = "GravNet.Result", -- server -> client: result of Link/Unlink/ForceSync (success, message)
	RewardSound = "GravNet.RewardSound", -- server -> client: play the reward sound
	Notify = "GravNet.Notify", -- server -> client: chat notice (superadmins: linked, out of sync; buyers: purchase delivered)
	StoreMenu = "GravNet.StoreMenu", -- server -> client: open the store menu (sh_store_menu.lua)
	SaveStoreMessages = "GravNet.SaveStoreMessages", -- client -> server: store chat message settings (enabled, minutes, message)
}
