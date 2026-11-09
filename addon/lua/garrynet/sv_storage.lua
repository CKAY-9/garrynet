-- Persists link state between restarts in garrysmod/data/garrynet/state.json.
-- The JWT is a secret: never send it to clients and never print it.

GarryNet.Storage = GarryNet.Storage or {}

local FILE = GarryNet.Config.DataDir .. "/state.json"

local State = {
	server_id = nil, -- number, assigned by the backend when linking
	token = nil, -- string, the server's rolling JWT
	server_name = nil, -- string, the server's name on GarryNet (display only)
}

-- Load persisted state from disk into memory. Called once on startup.
function GarryNet.Storage.Load()
	if not file.IsDir(GarryNet.Config.DataDir, "DATA") then
		file.CreateDir(GarryNet.Config.DataDir)
	end

	local raw = file.Read(FILE, "DATA")
	if not raw or raw == "" then return end

	local data = util.JSONToTable(raw)
	if not istable(data) then
		GarryNet.Util.Warn("Ignoring unreadable " .. FILE)
		return
	end

	State.server_id = tonumber(data.server_id)
	State.token = isstring(data.token) and data.token ~= "" and data.token or nil
	State.server_name = isstring(data.server_name) and data.server_name or nil
end

-- Write the in-memory state to disk.
function GarryNet.Storage.Save()
	if not file.IsDir(GarryNet.Config.DataDir, "DATA") then
		file.CreateDir(GarryNet.Config.DataDir)
	end

	if not GarryNet.Storage.IsLinked() then
		file.Delete(FILE)
		return
	end

	file.Write(FILE, util.TableToJSON({
		server_id = State.server_id,
		token = State.token,
		server_name = State.server_name,
	}))
end

function GarryNet.Storage.GetServerId()
	return State.server_id
end

function GarryNet.Storage.GetToken()
	return State.token
end

function GarryNet.Storage.GetServerName()
	return State.server_name
end

-- Replace the stored credentials (after linking or a JWT rotation) and persist them.
-- `server_name` is optional; the previous one is kept if it's nil.
function GarryNet.Storage.SetCredentials(server_id, token, server_name)
	State.server_id = server_id
	State.token = token
	if server_name ~= nil then State.server_name = server_name end
	GarryNet.Storage.Save()
end

-- Update the display name (it can be renamed on the website), persisting only on change.
function GarryNet.Storage.SetServerName(server_name)
	if not isstring(server_name) or server_name == State.server_name then return end
	State.server_name = server_name
	GarryNet.Storage.Save()
end

-- Forget the credentials (unlinking) and persist.
function GarryNet.Storage.Clear()
	State.server_id = nil
	State.token = nil
	State.server_name = nil
	GarryNet.Storage.Save()
end

function GarryNet.Storage.IsLinked()
	return State.server_id ~= nil and State.token ~= nil
end

GarryNet.Storage.Load()
