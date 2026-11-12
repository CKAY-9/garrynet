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
--     store_messages = {         -- repeating store chat messages (sv_store_messages.lua)
--         enabled = bool,
--         minutes = number,      -- between messages
--         message = string,
--         next_in = number|nil,  -- seconds until the next one, when enabled
--         skipped = string,      -- why the last one wasn't sent; "" if it was
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

	local messages = status.store_messages
	net.WriteBool(messages.enabled)
	net.WriteUInt(messages.minutes, 16)
	net.WriteString(messages.message)
	net.WriteBool(messages.next_in ~= nil)
	if messages.next_in then net.WriteUInt(math.Clamp(math.floor(messages.next_in), 0, 2 ^ 31), 32) end
	net.WriteString(messages.skipped or "")
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

	status.store_messages = {
		enabled = net.ReadBool(),
		minutes = net.ReadUInt(16),
		message = net.ReadString(),
	}
	if net.ReadBool() then status.store_messages.next_in = net.ReadUInt(32) end
	status.store_messages.skipped = net.ReadString()

	return status
end
