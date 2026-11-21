-- Thin wrapper around HTTP() for talking to the GravNet backend.
-- Backend routes used by the addon (docs/ADDON.md):
--   POST /gnet/server/link                  access key as the authorization header -> server JWT
--   *    /gnet/server/{SERVER_ID}/[route]   authorized with the server JWT

GravNet.HTTP = GravNet.HTTP or {}

-- Base URL from the gravnet_api_url convar, without a trailing slash. Returns "" if unset.
function GravNet.HTTP.GetBaseUrl()
	local cvar = GetConVar("gravnet_api_url")
	if not cvar then return "" end
	local url = string.Trim(cvar:GetString())
	return (string.gsub(url, "/+$", ""))
end

function GravNet.HTTP.IsConfigured()
	return GravNet.HTTP.GetBaseUrl() ~= ""
end

-- True when the backend is reached over plain HTTP on another machine, where the access key and
-- server token could be read or altered in transit. Loopback hosts are fine: nothing leaves the box.
function GravNet.HTTP.IsInsecure()
	local url = string.lower(GravNet.HTTP.GetBaseUrl())
	if url == "" or string.sub(url, 1, 8) == "https://" then return false end

	local rest = string.match(url, "^[%a][%w+.-]*://(.*)$") or url
	rest = string.match(rest, "^[^@/]*@(.*)$") or rest -- drop any user:password@
	local host = string.match(rest, "^(%[[^%]]*%])") or string.match(rest, "^([^/:?#]+)") or ""
	local loopback_v4 = string.match(host, "^127%.%d+%.%d+%.%d+$") ~= nil
	local localhost = host == "localhost" or string.sub(host, -10) == ".localhost"
	return not (localhost or host == "[::1]" or loopback_v4)
end

-- Whether plain HTTP to another machine was allowed for development (gravnet_allow_http 1).
local function HttpAllowed()
	local cvar = GetConVar("gravnet_allow_http")
	return cvar ~= nil and cvar:GetBool()
end

-- True when nothing may be sent to the backend: plain HTTP to another machine, not allowed for
-- development.
function GravNet.HTTP.IsBlocked()
	return GravNet.HTTP.IsInsecure() and not HttpAllowed()
end

-- What's wrong with gravnet_api_url's security, or nil if nothing is.
function GravNet.HTTP.InsecureMessage()
	if not GravNet.HTTP.IsInsecure() then return nil end
	if HttpAllowed() then
		return "gravnet_api_url doesn't use https://, so this server's access key and token are sent unencrypted. "
			.. "That's only allowed because gravnet_allow_http is 1, which is for development."
	end
	return "gravnet_api_url must be an https:// URL unless GravNet runs on this machine: this server's access "
		.. "key and token would be sent unencrypted. Nothing is sent to GravNet until it is."
end

-- Warn in the server console once the URL is known (server.cfg runs after autorun), and again
-- whenever either setting changes.
local function WarnIfInsecure()
	local message = GravNet.HTTP.InsecureMessage()
	if message then GravNet.Util.Warn(message) end
end

hook.Add("InitPostEntity", "GravNet.HTTP.InsecureWarning", WarnIfInsecure)
cvars.AddChangeCallback("gravnet_api_url", WarnIfInsecure, "GravNet.HTTP.InsecureWarning")
cvars.AddChangeCallback("gravnet_allow_http", WarnIfInsecure, "GravNet.HTTP.InsecureWarning")

-- A human-readable reason a request failed, for logs and the menu.
local function Describe(status, body, reason)
	if istable(body) and isstring(body.error) and body.error ~= "" then
		return body.error .. " (HTTP " .. status .. ")"
	end
	if status == 0 then
		return "GravNet could not be reached: " .. tostring(reason or "unknown error")
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
function GravNet.HTTP.Request(method, path, opts, callback)
	opts = opts or {}
	callback = callback or function() end

	local base = GravNet.HTTP.GetBaseUrl()
	if base == "" then
		callback(false, 0, nil, "gravnet_api_url is not set (see server.cfg)")
		return
	end
	if GravNet.HTTP.IsBlocked() then
		callback(false, 0, nil, "gravnet_api_url must be an https:// URL unless GravNet runs on this machine")
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
		timeout = GravNet.Config.HttpTimeout,

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
-- the response carries one. A 401 means the server and GravNet are out of sync; callers
-- decide what to do about it (see GravNet.Link.OnOutOfSync).
--   callback: same as GravNet.HTTP.Request, plus a 5th argument: whether the credentials used
--             are still the stored ones (false if the server re-linked or unlinked meanwhile).
function GravNet.HTTP.ServerRequest(method, route, body, callback)
	callback = callback or function() end

	if not GravNet.Storage.IsLinked() then
		callback(false, 0, nil, "This server is not linked to GravNet.", false)
		return
	end

	local server_id = GravNet.Storage.GetServerId()
	local token = GravNet.Storage.GetToken()

	GravNet.HTTP.Request(method, "/gnet/server/" .. server_id .. route, {
		body = body,
		headers = { Authorization = "Bearer " .. token },
	}, function(ok, status, data, err)
		local current = GravNet.Storage.GetServerId() == server_id and GravNet.Storage.GetToken() == token

		if ok and current and istable(data) and isstring(data.token) and data.token ~= "" then
			GravNet.Storage.SetCredentials(server_id, data.token)
		end

		callback(ok, status, data, err, current)
	end)
end
