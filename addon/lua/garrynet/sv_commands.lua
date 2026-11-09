-- Chat command: a superadmin saying !gnet gets the GarryNet menu.

hook.Add("PlayerSay", "GarryNet.Commands.ChatCommand", function(ply, text)
	if string.lower(string.Trim(text)) ~= GarryNet.Config.ChatCommand then return end

	if not ply:IsSuperAdmin() then
		-- TODO: tell the player they lack permission (chat message).
		return ""
	end

	GarryNet.OpenMenu(ply)
	return "" -- hide the command from chat
end)
