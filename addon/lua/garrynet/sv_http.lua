-- Thin wrapper around HTTP() for talking to the GarryNet backend.
-- Backend routes used by the addon (docs/ADDON.md):
--   POST /gnet/server/link                  access key as the authorization header -> server JWT
--   *    /gnet/server/{SERVER_ID}/[route]   authorized with the server JWT

GarryNet.HTTP = GarryNet.HTTP or {}

-- Base URL from the garrynet_api_url convar, without a trailing slash. Returns "" if unset.
function GarryNet.HTTP.GetBaseUrl()
	local cvar = GetConVar("garrynet_api_url")
	if not cvar then return "" end
	local url = string.Trim(cvar:GetString())
	return (string.gsub(url, "/+$", ""))
end

function GarryNet.HTTP.IsConfigured()
	return GarryNet.HTTP.GetBaseUrl() ~= ""
end

GarryNet.HTTP.InsecureWarning = "garrynet_api_url doesn't use https://, so this server's access key and token "
	.. "are sent unencrypted and can be stolen in transit. Use an https:// URL unless GarryNet is on a "
	.. "private network you fully trust."

-- True when the backend is reached over plain HTTP on another machine, where the access key and
-- server token could be read or altered in transit. Loopback hosts are fine: nothing leaves the box.
function GarryNet.HTTP.IsInsecure()
	local url = string.lower(GarryNet.HTTP.GetBaseUrl())
	if url == "" or string.sub(url, 1, 8) == "https://" then return false end

	local rest = string.match(url, "^[%a][%w+.-]*://(.*)$") or url
	rest = string.match(rest, "^[^@/]*@(.*)$") or rest -- drop any user:password@
	local host = string.match(rest, "^(%[[^%]]*%])") or string.match(rest, "^([^/:?#]+)") or ""
	local loopback_v4 = string.match(host, "^127%.%d+%.%d+%.%d+$") ~= nil
	return not (host == "localhost" or host == "[::1]" or loopback_v4)
end

-- Warn in the server console once the URL is known (server.cfg runs after autorun), and again
-- whenever it's changed to an insecure one.
local function WarnIfInsecure()
	if GarryNet.HTTP.IsInsecure() then GarryNet.Util.Warn(GarryNet.HTTP.InsecureWarning) end
end

hook.Add("InitPostEntity", "GarryNet.HTTP.InsecureWarning", WarnIfInsecure)
cvars.AddChangeCallback("garrynet_api_url", WarnIfInsecure, "GarryNet.HTTP.InsecureWarning")

-- A human-readable reason a request failed, for logs and the menu.
local function Describe(status, body, reason)
	if istable(body) and isstring(body.error) and body.error ~= "" then
		return body.error .. " (HTTP " .. status .. ")"
	end
	if status == 0 then
		return "GarryNet could not be reached: " .. tostring(reason or "unknown error")
	end
	return "HTTP " .. status
end

-- Low-level request.
--   method:   "GET" | "POST" | "PUT" | "DELETE"
--   path:     route starting with "/", appended to the base URL
--   opts:     { body = table|nil (sent as JSON), headers = table|nil }
--   callback: function(success: boolean, status: number, body: table|nil, err: string|nil)
--             `body` is the decoded JSON response, if any; `err` describes a failure.
--             `status` is 0 if no response was received.
function GarryNet.HTTP.Request(method, path, opts, callback)
	opts = opts or {}
	callback = callback or function() end

	local base = GarryNet.HTTP.GetBaseUrl()
	if base == "" then
		callback(false, 0, nil, "garrynet_api_url is not set (see server.cfg)")
		return
	end

	local headers = { Accept = "application/json" }
	for k, v in pairs(opts.headers or {}) do
		headers[k] = v
	end

	local request = {
		url = base .. path,
		method = method,
		headers = headers,
		timeout = GarryNet.Config.HttpTimeout,

		success = function(status, raw)
			local body = nil
			if isstring(raw) and raw ~= "" then body = util.JSONToTable(raw) end
			local ok = status >= 200 and status < 300
			callback(ok, status, body, not ok and Describe(status, body) or nil)
		end,

		failed = function(reason)
			callback(false, 0, nil, Describe(0, nil, reason))
		end,
	}

	if opts.body ~= nil then
		request.body = util.TableToJSON(opts.body)
		request.type = "application/json"
	elseif method ~= "GET" then
		-- Empty JSON object rather than no body, so the request carries a Content-Length.
		request.body = "{}"
		request.type = "application/json"
	end

	if not HTTP(request) then
		callback(false, 0, nil, "the HTTP request could not be started")
	end
end

-- Request to /gnet/server/{SERVER_ID}<route> using the stored JWT. Stores a rotated JWT when
-- the response carries one. A 401 means the server and GarryNet are out of sync; callers
-- decide what to do about it (see GarryNet.Link.OnOutOfSync).
--   callback: same as GarryNet.HTTP.Request, plus a 5th argument: whether the credentials used
--             are still the stored ones (false if the server re-linked or unlinked meanwhile).
function GarryNet.HTTP.ServerRequest(method, route, body, callback)
	callback = callback or function() end

	if not GarryNet.Storage.IsLinked() then
		callback(false, 0, nil, "This server is not linked to GarryNet.", false)
		return
	end

	local server_id = GarryNet.Storage.GetServerId()
	local token = GarryNet.Storage.GetToken()

	GarryNet.HTTP.Request(method, "/gnet/server/" .. server_id .. route, {
		body = body,
		headers = { Authorization = "Bearer " .. token },
	}, function(ok, status, data, err)
		local current = GarryNet.Storage.GetServerId() == server_id and GarryNet.Storage.GetToken() == token

		if ok and current and istable(data) and isstring(data.token) and data.token ~= "" then
			GarryNet.Storage.SetCredentials(server_id, data.token)
		end

		callback(ok, status, data, err, current)
	end)
end
