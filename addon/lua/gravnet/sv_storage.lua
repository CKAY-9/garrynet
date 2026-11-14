-- Persists link state between restarts in garrysmod/data/gravnet/state.json.
-- The JWT is a secret: never send it to clients and never print it.

GravNet.Storage = GravNet.Storage or {}

local FILE = GravNet.Config.DataDir .. "/state.json"

local State = {
	server_id = nil, -- number, assigned by the backend when linking
	token = nil, -- string, the server's rolling JWT
	server_name = nil, -- string, the server's name on GravNet (display only)
}

-- Load persisted state from disk into memory. Called once on startup.
function GravNet.Storage.Load()
	if not file.IsDir(GravNet.Config.DataDir, "DATA") then
		file.CreateDir(GravNet.Config.DataDir)
	end

	local raw = file.Read(FILE, "DATA")
	if not raw or raw == "" then return end

	local data = util.JSONToTable(raw)
	if not istable(data) then
		GravNet.Util.Warn("Ignoring unreadable " .. FILE)
		return
	end

	State.server_id = tonumber(data.server_id)
	State.token = isstring(data.token) and data.token ~= "" and data.token or nil
	State.server_name = isstring(data.server_name) and data.server_name or nil
end

-- Write the in-memory state to disk.
function GravNet.Storage.Save()
	if not file.IsDir(GravNet.Config.DataDir, "DATA") then
		file.CreateDir(GravNet.Config.DataDir)
	end

	if not GravNet.Storage.IsLinked() then
		file.Delete(FILE)
		return
	end

	file.Write(FILE, util.TableToJSON({
		server_id = State.server_id,
		token = State.token,
		server_name = State.server_name,
	}))
end

function GravNet.Storage.GetServerId()
	return State.server_id
end

function GravNet.Storage.GetToken()
	return State.token
end

function GravNet.Storage.GetServerName()
	return State.server_name
end

-- Replace the stored credentials (after linking or a JWT rotation) and persist them.
-- `server_name` is optional; the previous one is kept if it's nil.
function GravNet.Storage.SetCredentials(server_id, token, server_name)
	State.server_id = server_id
	State.token = token
	if server_name ~= nil then State.server_name = server_name end
	GravNet.Storage.Save()
end

-- Update the display name (it can be renamed on the website), persisting only on change.
function GravNet.Storage.SetServerName(server_name)
	if not isstring(server_name) or server_name == State.server_name then return end
	State.server_name = server_name
	GravNet.Storage.Save()
end

-- Forget the credentials (unlinking) and persist.
function GravNet.Storage.Clear()
	State.server_id = nil
	State.token = nil
	State.server_name = nil
	GravNet.Storage.Save()
end

function GravNet.Storage.IsLinked()
	return State.server_id ~= nil and State.token ~= nil
end

GravNet.Storage.Load()
