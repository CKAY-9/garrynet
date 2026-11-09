-- Persists link state between restarts in garrysmod/data/garrynet/.
-- The JWT is a secret: never send it to clients and never print it.

GarryNet.Storage = GarryNet.Storage or {}

local State = {
	server_id = nil, -- number, assigned by the backend when linking
	token = nil, -- string, the server's rolling JWT
}

-- Load persisted state from disk into memory. Called once on startup.
function GarryNet.Storage.Load()
	-- TODO: create GarryNet.Config.DataDir if missing, read state.json, populate State.
end

-- Write the in-memory state to disk.
function GarryNet.Storage.Save()
	-- TODO: serialize State to garrynet/state.json.
end

function GarryNet.Storage.GetServerId()
	return State.server_id
end

function GarryNet.Storage.GetToken()
	return State.token
end

-- Replace the stored credentials (e.g. after linking or after a JWT rotation) and persist them.
function GarryNet.Storage.SetCredentials(server_id, token)
	State.server_id = server_id
	State.token = token
	GarryNet.Storage.Save()
end

-- Forget the credentials (unlinking) and persist.
function GarryNet.Storage.Clear()
	State.server_id = nil
	State.token = nil
	GarryNet.Storage.Save()
end

function GarryNet.Storage.IsLinked()
	return State.server_id ~= nil and State.token ~= nil
end

GarryNet.Storage.Load()
