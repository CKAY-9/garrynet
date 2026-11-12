-- Chat commands: a superadmin saying !gnet gets the GarryNet menu; anyone saying !gstore gets
-- the store menu. Console commands do what !gnet does from the server console (e.g. a dedicated
-- server with nobody on).

hook.Add("PlayerSay", "GarryNet.Commands.ChatCommand", function(ply, text)
	local command = string.lower(string.Trim(text))

	if command == GarryNet.Config.StoreCommand then
		GarryNet.StoreMenu.Open(ply)
		return "" -- hide the command from chat
	end

	if command ~= GarryNet.Config.ChatCommand then return end

	if not ply:IsSuperAdmin() then
		ply:ChatPrint("[GarryNet] Only superadmins can use " .. GarryNet.Config.ChatCommand .. ".")
		return ""
	end

	GarryNet.OpenMenu(ply)
	return "" -- hide the command from chat
end)

-- Console commands run from the server console, or by a superadmin from their own console.
local function Command(name, help, run)
	concommand.Add(name, function(ply, _, _, arg_str)
		local from_console = not IsValid(ply)
		if not from_console and not ply:IsSuperAdmin() then return end

		local function reply(message)
			if from_console then
				GarryNet.Util.Log(message)
			elseif IsValid(ply) then
				ply:PrintMessage(HUD_PRINTCONSOLE, "[GarryNet] " .. message)
			end
		end

		run(reply, string.Trim(arg_str or ""))
	end, nil, help)
end

Command("garrynet_status", "Show whether this server is linked to GarryNet.", function(reply)
	local status = GarryNet.GetStatus()
	if not status.configured then reply("garrynet_api_url is not set.") end
	if status.insecure then reply("Warning: " .. GarryNet.HTTP.InsecureWarning) end

	if not status.linked then
		reply("Not linked.")
		return
	end

	reply(string.format("Linked as server #%d (%s).", status.server_id, status.server_name))
	if status.last_sync then
		reply(string.format("Last sync %ds ago: %s %s", status.last_sync.ago,
			status.last_sync.ok and "OK." or "FAILED.", status.last_sync.message))
	else
		reply("No sync yet.")
	end

	local store = GarryNet.Store.GetLastSync()
	if store then
		reply(string.format("Last store sync %ds ago: %s", os.time() - store.time, store.message))
	end
	local bans = GarryNet.Bans.GetLastSync()
	if bans then
		reply(string.format("Last ban list sync %ds ago: %s", os.time() - bans.time, bans.message))
	end
	local messages = GarryNet.StoreMessages.GetStatus()
	if messages.enabled then
		reply(string.format("Store chat messages: every %d min%s", messages.minutes,
			messages.skipped ~= "" and (", last one not sent: " .. messages.skipped) or "."))
	else
		reply("Store chat messages: off.")
	end
	reply("Usergroups are managed with " .. GarryNet.Rewards.Adapter().name .. ".")
end)

-- Reply with the outcome and refresh any open menus.
local function Done(reply)
	return function(_, message)
		reply(message)
		GarryNet.BroadcastStatus()
	end
end

Command("garrynet_link", "Link this server to GarryNet: garrynet_link <access key>", function(reply, args)
	-- Link() warns in the server console; a superadmin running this from their own console
	-- needs to see it too.
	if GarryNet.HTTP.IsInsecure() then reply("Warning: " .. GarryNet.HTTP.InsecureWarning) end
	GarryNet.Link.Link(args, function(success, message)
		Done(reply)(success, message)
		if success then GarryNet.NotifyAdmins("This server was linked from the console: " .. message, true) end
	end)
end)

Command("garrynet_unlink", "Unlink this server from GarryNet.", function(reply)
	GarryNet.Link.Unlink(Done(reply))
end)

Command("garrynet_sync", "Send a heartbeat to GarryNet now.", function(reply)
	GarryNet.Interval.Run(Done(reply))
end)

Command("garrynet_store_sync", "Fetch and deliver store purchases now.", function(reply)
	GarryNet.Store.Sync(nil, function(_, message) reply(message) end)
end)

Command("garrynet_store_grants", "Show a player's store usergroups and weapons: garrynet_store_grants <SteamID64>", function(reply, args)
	if not string.match(args, "^%d+$") then
		reply("Usage: garrynet_store_grants <SteamID64>")
		return
	end
	local grants, applied = GarryNet.Rewards.GetGrants(args)
	if #grants == 0 and not applied then
		reply("No store usergroups or weapons for " .. args .. ".")
		return
	end
	local group, expires = GarryNet.Rewards.GroupExpiry(args)
	if group then
		reply(string.format("Usergroup %s, %s.", group, expires == math.huge and "permanent"
			or ("until " .. os.date("!%Y-%m-%d %H:%M UTC", expires))))
	end
	for _, g in ipairs(grants) do
		if g.kind == "usergroup" then
			reply(string.format("  purchase #%d: %s for %s", g.fulfillment_id, g.usergroup,
				g.duration and (g.duration .. "s") or "ever"))
		else
			reply(string.format("  purchase #%d: weapon %s", g.fulfillment_id, g.class))
		end
	end
end)
