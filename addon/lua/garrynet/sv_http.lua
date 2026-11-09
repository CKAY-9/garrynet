-- Thin wrapper around HTTP() for talking to the GarryNet backend.
-- Backend routes used by the addon (docs/ADDON.md):
--   POST /gnet/server/link                  access key as the authorization header -> server JWT
--   *    /gnet/server/{SERVER_ID}/[route]   authorized with the server JWT

GarryNet.HTTP = GarryNet.HTTP or {}

-- Base URL from the garrynet_api_url convar, without a trailing slash. Returns "" if unset.
function GarryNet.HTTP.GetBaseUrl()
	-- TODO: read GetConVar("garrynet_api_url"), trim trailing slashes.
	return ""
end

-- Low-level request.
--   method:   "GET" | "POST" | "PUT" | "DELETE"
--   path:     route starting with "/", appended to the base URL
--   opts:     { body = table|nil (sent as JSON), headers = table|nil }
--   callback: function(success: boolean, status: number, body: table|nil)
--             `body` is the decoded JSON response, if any.
function GarryNet.HTTP.Request(method, path, opts, callback)
	-- TODO: build the request, JSON encode the body, call HTTP(), decode the response,
	-- and invoke callback. Fail (callback(false, 0, nil)) if the base URL is unset.
	if callback then callback(false, 0, nil) end
end

-- Request to /gnet/server/{SERVER_ID}<route> using the stored JWT.
-- Should handle a rotated JWT in the response and a 401 (treat as unlinked/out of sync).
function GarryNet.HTTP.ServerRequest(method, route, body, callback)
	-- TODO: requires GarryNet.Storage.IsLinked(); set the authorization header to the JWT.
	if callback then callback(false, 0, nil) end
end
