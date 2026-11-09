-- Linking and unlinking the server (docs/ADDON.md, docs/USER_STORIES.md "Linking a Garry's Mod Server").
-- Linking exchanges the one-time access key for a rolling server JWT; the key is then revoked backend-side.

GarryNet.Link = GarryNet.Link or {}

-- Exchange `access_key` for a server JWT.
--   POST /gnet/server/link with the key as the authorization header.
--   On success store the server id and JWT via GarryNet.Storage.SetCredentials and start the interval.
--   callback: function(success: boolean, message: string)
function GarryNet.Link.Link(access_key, callback)
	-- TODO: validate format with GarryNet.Util.IsValidAccessKeyFormat, call the backend, store credentials.
	if callback then callback(false, "Not implemented") end
end

-- Unlink: remove the stored JWT (and tell the backend to revoke it / the access key if it still exists).
--   callback: function(success: boolean, message: string)
function GarryNet.Link.Unlink(callback)
	-- TODO: call the backend revoke route (route TBD), then GarryNet.Storage.Clear() and stop the interval.
	if callback then callback(false, "Not implemented") end
end

function GarryNet.Link.IsLinked()
	return GarryNet.Storage.IsLinked()
end
