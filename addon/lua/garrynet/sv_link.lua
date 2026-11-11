-- Linking and unlinking the server (docs/ADDON.md, docs/USER_STORIES.md "Linking a Garry's Mod Server").
-- Linking exchanges the one-time access key for a rolling server JWT; the key is then revoked backend-side.

GarryNet.Link = GarryNet.Link or {}

-- True while a link or unlink request is in flight.
local Busy = false

function GarryNet.Link.IsBusy()
	return Busy
end

function GarryNet.Link.IsLinked()
	return GarryNet.Storage.IsLinked()
end

-- Exchange `access_key` for a server JWT.
--   POST /gnet/server/link with the key as the authorization header.
--   On success store the server id and JWT and start the interval, sending the first heartbeat
--   right away.
--   callback: function(success: boolean, message: string)
function GarryNet.Link.Link(access_key, callback)
	callback = callback or function() end
	access_key = string.Trim(tostring(access_key or ""))

	if GarryNet.Storage.IsLinked() then
		callback(false, "This server is already linked. Unlink it first.")
		return
	end
	if Busy then
		callback(false, "A link request is already in progress.")
		return
	end
	if not GarryNet.Util.IsValidAccessKeyFormat(access_key) then
		callback(false, "That isn't an access key. Keys look like gnet_ followed by 64 hex characters.")
		return
	end
	if not GarryNet.HTTP.IsConfigured() then
		callback(false, "garrynet_api_url is not set. Add it to server.cfg, e.g. garrynet_api_url \"https://api.example.com\".")
		return
	end

	if GarryNet.HTTP.IsInsecure() then GarryNet.Util.Warn(GarryNet.HTTP.InsecureWarning) end

	Busy = true
	GarryNet.HTTP.Request("POST", "/gnet/server/link", {
		headers = { Authorization = "Bearer " .. access_key },
	}, function(ok, status, data, err)
		Busy = false

		if status == 401 then
			callback(false, "GarryNet rejected the access key. Keys only work once; generate a new one on the website.")
			return
		end

		local server_id = istable(data) and tonumber(data.server_id)
		local token = istable(data) and data.token
		if not ok or not server_id or not isstring(token) or token == "" then
			callback(false, "Linking failed: " .. (err or "unexpected response from GarryNet"))
			return
		end

		local name = istable(data.server) and isstring(data.server.name) and data.server.name or ""
		GarryNet.Storage.SetCredentials(server_id, token, name)
		GarryNet.Util.Log(string.format("Linked to GarryNet as server #%d (%s)", server_id, name))

		GarryNet.Interval.OnLinked()
		GarryNet.Bans.Sync()
		callback(true, string.format("Linked to GarryNet as \"%s\" (#%d).", name, server_id))
	end)
end

-- Unlink: ask the backend to revoke the JWT (and the access key, if it still exists), then
-- forget the credentials. The credentials are forgotten even if GarryNet can't be reached.
--   callback: function(success: boolean, message: string)
function GarryNet.Link.Unlink(callback)
	callback = callback or function() end

	if not GarryNet.Storage.IsLinked() then
		callback(false, "This server is not linked.")
		return
	end
	if Busy then
		callback(false, "A link request is already in progress.")
		return
	end

	Busy = true
	GarryNet.HTTP.ServerRequest("POST", "/unlink", nil, function(ok, status, _, err)
		Busy = false
		GarryNet.Link.Forget()

		-- 401: GarryNet had already revoked the token, so the server is unlinked there too.
		if ok or status == 401 then
			GarryNet.Util.Log("Unlinked from GarryNet")
			callback(true, "Unlinked from GarryNet. Generate a new access key on the website to link again.")
		else
			GarryNet.Util.Warn("Unlinked locally, but GarryNet was not told: " .. tostring(err))
			callback(true, "Unlinked locally, but GarryNet was not told (" .. tostring(err)
				.. "). Regenerate the access key on the website to revoke the old token.")
		end
	end)
end

-- Drop the credentials and stop syncing. The ban list goes too: it can't be kept up to date.
function GarryNet.Link.Forget()
	GarryNet.Interval.Stop()
	GarryNet.Bans.Clear()
	GarryNet.Storage.Clear()
	GarryNet.BroadcastStatus()
end

-- GarryNet answered 401: the server was unlinked, deleted, or its access key regenerated on
-- the website (or its token expired). The stored credentials are useless now.
function GarryNet.Link.OnOutOfSync()
	if not GarryNet.Storage.IsLinked() then return end

	GarryNet.Link.Forget()
	local message = "GarryNet no longer accepts this server's token, so it has been unlinked. "
		.. "Generate a new access key on the website and link again with " .. GarryNet.Config.ChatCommand .. "."
	GarryNet.Util.Warn(message)
	GarryNet.NotifyAdmins(message, false)
end
