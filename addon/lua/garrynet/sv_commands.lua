-- Chat command: a superadmin saying !gnet gets the GarryNet menu.
-- Console commands do the same from the server console (e.g. a dedicated server with nobody on).

hook.Add("PlayerSay", "GarryNet.Commands.ChatCommand", function(ply, text)
	if string.lower(string.Trim(text)) ~= GarryNet.Config.ChatCommand then return end

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
end)

-- Reply with the outcome and refresh any open menus.
local function Done(reply)
	return function(_, message)
		reply(message)
		GarryNet.BroadcastStatus()
	end
end

Command("garrynet_link", "Link this server to GarryNet: garrynet_link <access key>", function(reply, args)
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
