-- GarryNet bootstrap. Creates the global table and loads every module in order.
-- Module files live in lua/garrynet/ and are prefixed by realm: sh_ (shared), sv_ (server), cl_ (client).

GarryNet = GarryNet or {}
GarryNet.Version = "0.1.0"

local BASE = "garrynet/"

-- Load order matters: later files may depend on earlier ones.
local FILES = {
	"sh_config.lua",
	"sh_util.lua",

	"sv_storage.lua",
	"sv_http.lua",
	"sv_link.lua",
	"sv_interval.lua",
	"sv_net.lua",
	"sv_commands.lua",

	"cl_net.lua",
	"cl_menu.lua",
}

function GarryNet.Include(file)
	local prefix = string.sub(file, 1, 3)
	local path = BASE .. file

	if prefix == "sh_" then
		if SERVER then AddCSLuaFile(path) end
		include(path)
	elseif prefix == "sv_" then
		if SERVER then include(path) end
	elseif prefix == "cl_" then
		if SERVER then
			AddCSLuaFile(path)
		else
			include(path)
		end
	else
		ErrorNoHalt("[GarryNet] Unknown realm prefix for file: " .. file .. "\n")
	end
end

for _, file in ipairs(FILES) do
	GarryNet.Include(file)
end

if SERVER then
	MsgC(Color(59, 130, 246), "[GarryNet] ", color_white, "Loaded v" .. GarryNet.Version .. "\n")
end
