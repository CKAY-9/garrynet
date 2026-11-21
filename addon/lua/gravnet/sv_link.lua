-- Linking and unlinking the server (docs/ADDON.md, docs/USER_STORIES.md "Linking a Garry's Mod Server").
-- Linking exchanges the one-time access key for a rolling server JWT; the key is then revoked backend-side.

GravNet.Link = GravNet.Link or {}

-- True while a link or unlink request is in flight.
local Busy = false

function GravNet.Link.IsBusy()
	return Busy
end

function GravNet.Link.IsLinked()
	return GravNet.Storage.IsLinked()
end

-- Exchange `access_key` for a server JWT.
--   POST /gnet/server/link with the key as the authorization header.
--   On success store the server id and JWT and start the interval, sending the first heartbeat
--   right away.
--   callback: function(success: boolean, message: string)
function GravNet.Link.Link(access_key, callback)
	callback = callback or function() end
	access_key = string.Trim(tostring(access_key or ""))

	if GravNet.Storage.IsLinked() then
		callback(false, "This server is already linked. Unlink it first.")
		return
	end
	if Busy then
		callback(false, "A link request is already in progress.")
		return
	end
	if not GravNet.Util.IsValidAccessKeyFormat(access_key) then
		callback(false, "That isn't an access key. Keys look like gnet_ followed by 64 hex characters.")
		return
	end
	if not GravNet.HTTP.IsConfigured() then
		callback(false, "gravnet_api_url is not set. Add it to server.cfg, e.g. gravnet_api_url \"https://api.example.com\".")
		return
	end

	if GravNet.HTTP.IsBlocked() then
		callback(false, GravNet.HTTP.InsecureMessage())
		return
	end
	local insecure = GravNet.HTTP.InsecureMessage()
	if insecure then GravNet.Util.Warn(insecure) end

	Busy = true
	GravNet.HTTP.Request("POST", "/gnet/server/link", {
		headers = { Authorization = "Bearer " .. access_key },
	}, function(ok, status, data, err)
		Busy = false

		if status == 401 then
			callback(false, "GravNet rejected the access key. Keys only work once; generate a new one on the website.")
			return
		end

		local server_id = istable(data) and tonumber(data.server_id)
		local token = istable(data) and data.token
		if not ok or not server_id or not isstring(token) or token == "" then
			callback(false, "Linking failed: " .. (err or "unexpected response from GravNet"))
			return
		end

		local name = istable(data.server) and isstring(data.server.name) and data.server.name or ""
		GravNet.Storage.SetCredentials(server_id, token, name)
		GravNet.Util.Log(string.format("Linked to GravNet as server #%d (%s)", server_id, name))

		GravNet.Interval.OnLinked()
		GravNet.Bans.Sync()
		callback(true, string.format("Linked to GravNet as \"%s\" (#%d).", name, server_id))
	end)
end

-- Unlink: ask the backend to revoke the JWT (and the access key, if it still exists), then
-- forget the credentials. The credentials are forgotten even if GravNet can't be reached.
--   callback: function(success: boolean, message: string)
function GravNet.Link.Unlink(callback)
	callback = callback or function() end

	if not GravNet.Storage.IsLinked() then
		callback(false, "This server is not linked.")
		return
	end
	if Busy then
		callback(false, "A link request is already in progress.")
		return
	end

	Busy = true
	GravNet.HTTP.ServerRequest("POST", "/unlink", nil, function(ok, status, _, err)
		Busy = false
		GravNet.Link.Forget()

		-- 401: GravNet had already revoked the token, so the server is unlinked there too.
		if ok or status == 401 then
			GravNet.Util.Log("Unlinked from GravNet")
			callback(true, "Unlinked from GravNet. Generate a new access key on the website to link again.")
		else
			GravNet.Util.Warn("Unlinked locally, but GravNet was not told: " .. tostring(err))
			callback(true, "Unlinked locally, but GravNet was not told (" .. tostring(err)
				.. "). Regenerate the access key on the website to revoke the old token.")
		end
	end)
end

-- Drop the credentials and stop syncing. The ban list goes too: it can't be kept up to date.
function GravNet.Link.Forget()
	GravNet.Interval.Stop()
	GravNet.Bans.Clear()
	GravNet.Storage.Clear()
	GravNet.BroadcastStatus()
end

-- GravNet answered 401: the server was unlinked, deleted, or its access key regenerated on
-- the website (or its token expired). The stored credentials are useless now.
function GravNet.Link.OnOutOfSync()
	if not GravNet.Storage.IsLinked() then return end

	GravNet.Link.Forget()
	local message = "GravNet no longer accepts this server's token, so it has been unlinked. "
		.. "Generate a new access key on the website and link again with " .. GravNet.Config.ChatCommand .. "."
	GravNet.Util.Warn(message)
	GravNet.NotifyAdmins(message, false)
end
