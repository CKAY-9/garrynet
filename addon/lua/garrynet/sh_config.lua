-- Shared constants. Nothing here talks to the backend.

GarryNet.Config = {
	-- Seconds between interval syncs (docs/ADDON.md: every minute).
	IntervalSeconds = 60,

	-- HTTP request timeout in seconds.
	HttpTimeout = 15,

	-- Chat command that opens the menu (superadmins only).
	ChatCommand = "!gnet",

	-- Directory (inside garrysmod/data) where link state is persisted.
	DataDir = "garrynet",

	-- Server access keys look like "gnet_" followed by 64 hex characters (docs/SERVERS.md).
	AccessKeyPattern = "^gnet_%x+$",
	AccessKeyLength = 69,
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
	Notify = "GarryNet.Notify", -- server -> client: chat notice for superadmins (linked, out of sync, ...)
}
