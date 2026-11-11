-- Gamemode stats (docs/ADDON.md "Gamemode Stats"), sent with each player in the heartbeat and
-- stored in their UserPlayer's `stats` (docs/USERS.md).
--
-- Two kinds:
--   values:   sampled from the player every heartbeat (and when they leave), replacing what
--             GarryNet has, e.g. darkrp_money.
--   counters: events counted as they happen with GarryNet.Stats.Count, added to what GarryNet
--             has, e.g. sandbox_props_spawned. Kept until a heartbeat is accepted (sv_interval).
--
-- Keys are lowercase, prefixed with the gamemode's name (as GarryNet shows it, e.g. "DarkRP"),
-- so the website can label `darkrp_money` as "Money".

GarryNet.Stats = GarryNet.Stats or {}

-- Providers by id: { active = function() -> bool, sample = function(ply) -> { key = value } | nil }
GarryNet.Stats.Providers = GarryNet.Stats.Providers or {}
local Providers = GarryNet.Stats.Providers

local KEY_PATTERN = "^[a-z0-9_]+$"

-- Add (or replace) a stats provider. Other addons can register their own gamemode's stats:
--   GarryNet.Stats.Register("murder", {
--       active = function() return engine.ActiveGamemode() == "murder" end,
--       sample = function(ply) return { murder_wins = ply:GetNWInt("Wins") } end,
--   })
-- `sample` is optional: a provider may only count events with GarryNet.Stats.Count.
function GarryNet.Stats.Register(id, provider)
	Providers[id] = provider
end

local function IsActive(provider)
	if not isfunction(provider.active) then return true end
	local ok, active = pcall(provider.active)
	return ok and active == true
end

-- The latest values of every active provider for a player. A provider that errors is skipped
-- (and logged) so one broken stat doesn't stop the heartbeat.
function GarryNet.Stats.Sample(ply)
	local values = {}
	if not IsValid(ply) then return values end

	for id, provider in pairs(Providers) do
		if isfunction(provider.sample) and IsActive(provider) then
			local ok, result = pcall(provider.sample, ply)
			if not ok then
				GarryNet.Util.Warn("Stats provider \"" .. tostring(id) .. "\" failed: " .. tostring(result))
			elseif istable(result) then
				for key, value in pairs(result) do
					if isstring(key) and string.match(key, KEY_PATTERN)
						and (isnumber(value) or isstring(value) or isbool(value)) then
						values[key] = value
					end
				end
			end
		end
	end
	return values
end

-- Count an event for a player, e.g. GarryNet.Stats.Count(ply, "sandbox_props_spawned").
function GarryNet.Stats.Count(ply, key, amount)
	if not isstring(key) or not string.match(key, KEY_PATTERN) then return end
	GarryNet.Interval.Count(ply, key, amount or 1)
end

-- DarkRP: money and current (last, once they leave) job.
local function DarkRPActive()
	return istable(DarkRP) and isfunction(FindMetaTable("Player").getDarkRPVar)
end

GarryNet.Stats.Register("darkrp", {
	active = DarkRPActive,
	sample = function(ply)
		local job = team.GetName(ply:Team())
		return {
			-- nil until DarkRP has loaded the player's data; the last known value is kept.
			darkrp_money = ply:getDarkRPVar("money"),
			darkrp_job = (job ~= "" and job) or nil,
		}
	end,
})

-- Sandbox: props spawned and tool gun uses. Only on Sandbox itself, not gamemodes derived from
-- it (DarkRP has its own stats).
local function SandboxActive()
	return engine.ActiveGamemode() == "sandbox"
end

GarryNet.Stats.Register("sandbox", { active = SandboxActive })

hook.Add("PlayerSpawnedProp", "GarryNet.Stats.Sandbox", function(ply)
	if SandboxActive() then GarryNet.Stats.Count(ply, "sandbox_props_spawned") end
end)

-- CanTool runs for every tool gun click (left, right, reload) that reaches the gamemode; a
-- click another addon refuses first isn't counted. Must not return a value.
hook.Add("CanTool", "GarryNet.Stats.Sandbox", function(ply)
	if SandboxActive() then GarryNet.Stats.Count(ply, "sandbox_tools_used") end
end)
