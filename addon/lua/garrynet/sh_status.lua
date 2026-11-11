-- The link status the server sends to superadmins for the !gnet menu. Shared so both realms
-- read and write it the same way. Never contains the JWT.
--
-- status = {
--     configured = bool,         -- garrynet_api_url is set
--     insecure = bool,           -- garrynet_api_url is plain HTTP to another machine
--     linked = bool,
--     server_id = number|nil,    -- when linked
--     server_name = string,      -- when linked; "" if unknown
--     busy = bool,               -- a link/unlink/sync request is in flight
--     last_sync = nil | {        -- latest sync attempt since the map started
--         ago = number,          -- seconds before the status was sent
--         ok = bool,
--         message = string,
--     },
-- }

GarryNet.Status = GarryNet.Status or {}

function GarryNet.Status.Write(status)
	net.WriteBool(status.configured)
	net.WriteBool(status.insecure == true)
	net.WriteBool(status.linked)
	net.WriteBool(status.busy)

	if status.linked then
		net.WriteUInt(status.server_id or 0, 32)
		net.WriteString(status.server_name or "")
	end

	net.WriteBool(status.last_sync ~= nil)
	if status.last_sync then
		net.WriteUInt(math.Clamp(math.floor(status.last_sync.ago), 0, 2 ^ 31), 32)
		net.WriteBool(status.last_sync.ok)
		net.WriteString(status.last_sync.message or "")
	end
end

function GarryNet.Status.Read()
	local status = {}
	status.configured = net.ReadBool()
	status.insecure = net.ReadBool()
	status.linked = net.ReadBool()
	status.busy = net.ReadBool()

	if status.linked then
		status.server_id = net.ReadUInt(32)
		status.server_name = net.ReadString()
	end

	if net.ReadBool() then
		status.last_sync = {
			ago = net.ReadUInt(32),
			ok = net.ReadBool(),
			message = net.ReadString(),
		}
	end

	return status
end
