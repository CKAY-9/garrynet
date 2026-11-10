-- Store rewards in-game (docs/STORE.md "Rewards"): usergroups (permanent or timed), DarkRP money
-- and permanent weapons. Lasting rewards are remembered per player in
-- garrysmod/data/garrynet/store_grants.json, keyed by the fulfillment that granted them, so a
-- refund takes exactly that purchase back and timed usergroups can expire.
--
-- Usergroups are set through an adapter: ULX (persisted by ULib) when it's installed, otherwise
-- plain Player:SetUserGroup, which GarryNet re-applies whenever the player joins. Other admin
-- mods can plug in with GarryNet.Rewards.SetUsergroupAdapter.
--
-- Staff are never demoted: a store usergroup only replaces a group listed in
-- GarryNet.Config.Store.OverridableGroups, or one the store set itself.

GarryNet.Rewards = GarryNet.Rewards or {}
local R = GarryNet.Rewards

local FILE = GarryNet.Config.DataDir .. "/store_grants.json"

local function Cfg() return GarryNet.Config.Store end

-- Per SteamID64:
--   applied = the store usergroup GarryNet last put the player in, or nil
--   grants  = { { fulfillment_id, kind = "usergroup" | "weapon", usergroup, duration, class, granted_at } }
--             `duration` is in seconds; nil means permanent.
local Players = {}

-- Saved as a list: util.JSONToTable turns numeric-looking keys (SteamID64s) into numbers,
-- which loses precision.
function R.Save()
	local list = {}
	for steam_id, entry in pairs(Players) do
		list[#list + 1] = { steam_id = steam_id, applied = entry.applied, grants = entry.grants }
	end
	if not file.IsDir(GarryNet.Config.DataDir, "DATA") then file.CreateDir(GarryNet.Config.DataDir) end
	file.Write(FILE, util.TableToJSON(list, true))
end

local function Load()
	local raw = file.Read(FILE, "DATA")
	if not raw or raw == "" then return end
	local list = util.JSONToTable(raw)
	if not istable(list) then
		GarryNet.Util.Warn("Ignoring unreadable " .. FILE)
		return
	end
	for _, item in ipairs(list) do
		if istable(item) and isstring(item.steam_id) and istable(item.grants) then
			Players[item.steam_id] = {
				applied = isstring(item.applied) and item.applied or nil,
				grants = item.grants,
			}
		end
	end
end

local function Entry(steam_id, create)
	local entry = Players[steam_id]
	if not entry and create then
		entry = { applied = nil, grants = {} }
		Players[steam_id] = entry
	end
	return entry
end

-- The online player with this SteamID64, or nil.
local function Online(steam_id)
	local ply = player.GetBySteamID64(steam_id)
	if IsValid(ply) then return ply end
end

-- Usergroup adapters ---------------------------------------------------------------------------
--   name        shown in logs and error messages
--   GroupExists function(group) -> bool
--   Get         function(steam_id, ply|nil) -> current group, or nil if unknown (player offline)
--   Set         function(steam_id, ply|nil, group) -> true if done, false if it must wait for
--               the player to join

local Plain = {
	name = "Garry's Mod",
	GroupExists = function() return true end,
	Get = function(_, ply)
		if IsValid(ply) then return ply:GetUserGroup() end
	end,
	Set = function(_, ply, group)
		if not IsValid(ply) then return false end
		ply:SetUserGroup(group)
		return true
	end,
}

local ULX = {
	name = "ULX",
	Available = function()
		return istable(ULib) and istable(ULib.ucl) and isfunction(ULib.ucl.addUser) and isfunction(ULib.ucl.removeUser)
	end,
	GroupExists = function(group)
		return ULib.ucl.groups[group] ~= nil
	end,
	Get = function(steam_id, ply)
		if IsValid(ply) then return ply:GetUserGroup() end
		local info = ULib.ucl.users[util.SteamIDFrom64(steam_id)]
		return (info and info.group) or Cfg().DefaultGroup
	end,
	Set = function(steam_id, ply, group)
		local id = util.SteamIDFrom64(steam_id)
		local info = ULib.ucl.users[id]
		-- Keep any per-user allows/denies an admin gave the player.
		local custom = info and ((istable(info.allow) and next(info.allow)) or (istable(info.deny) and next(info.deny)))
		if group == Cfg().DefaultGroup and info and not custom then
			ULib.ucl.removeUser(id)
		elseif group ~= Cfg().DefaultGroup or info then
			ULib.ucl.addUser(id, info and info.allow, info and info.deny, group)
		end
		if IsValid(ply) and ply:GetUserGroup() ~= group then ply:SetUserGroup(group) end
		return true
	end,
}

local Custom = nil

-- Use another admin mod's usergroups (SAM, ServerGuard, ...). `adapter` has the fields above.
function R.SetUsergroupAdapter(adapter)
	Custom = adapter
end

function R.Adapter()
	if Custom then return Custom end
	if ULX.Available() then return ULX end
	return Plain
end

-- Usergroup bookkeeping ------------------------------------------------------------------------

local function Contains(list, value)
	for _, v in ipairs(list or {}) do
		if v == value then return true end
	end
	return false
end

local function IsOverridable(group)
	return Contains(Cfg().OverridableGroups, group)
end

local function IsProtected(group)
	return Contains(Cfg().ProtectedGroups, string.lower(group))
end

-- For each granted usergroup: when it runs out (math.huge if permanent) and when it was last
-- bought. Purchases of the same group stack: each duration starts when the previous one ends.
local function GroupTimes(entry)
	local by_group = {}
	for _, g in ipairs(entry.grants) do
		if g.kind == "usergroup" then
			by_group[g.usergroup] = by_group[g.usergroup] or {}
			table.insert(by_group[g.usergroup], g)
		end
	end

	local times = {}
	for group, grants in pairs(by_group) do
		table.sort(grants, function(a, b)
			if a.granted_at ~= b.granted_at then return a.granted_at < b.granted_at end
			return a.fulfillment_id < b.fulfillment_id
		end)
		local expires, last = 0, 0
		for _, g in ipairs(grants) do
			if g.duration == nil then
				expires = math.huge
			else
				expires = math.max(expires, g.granted_at) + g.duration
			end
			last = math.max(last, g.granted_at)
		end
		times[group] = { expires = expires, last = last }
	end
	return times
end

-- The usergroup the player should have from the store: the most recently bought one still
-- running, or nil.
local function DesiredGroup(times, now)
	local best, best_last = nil, -1
	for group, t in pairs(times) do
		if t.expires > now and t.last > best_last then
			best, best_last = group, t.last
		end
	end
	return best
end

-- When the player's store usergroup runs out: nil if they have none, math.huge if permanent.
function R.GroupExpiry(steam_id)
	local entry = Players[steam_id]
	if not entry then return end
	local times = GroupTimes(entry)
	local group = DesiredGroup(times, os.time())
	return group, group and times[group].expires
end

-- Bring the player's usergroup in line with their grants: drops expired grants, sets the
-- desired store group, or puts them back in the default group once none is left. Players in a
-- group the store doesn't manage (staff) are left alone. Unless `quiet`, an online player is
-- told when their store usergroup ends.
function R.Reconcile(steam_id, quiet)
	local entry = Players[steam_id]
	if not entry then return end

	local now = os.time()
	local times = GroupTimes(entry)
	local changed = false

	local kept = {}
	for _, g in ipairs(entry.grants) do
		if g.kind ~= "usergroup" or times[g.usergroup].expires > now then
			kept[#kept + 1] = g
		else
			changed = true
		end
	end
	entry.grants = kept

	local desired = DesiredGroup(times, now)
	local adapter = R.Adapter()
	local ply = Online(steam_id)
	local current = adapter.Get(steam_id, ply)

	if current ~= nil and (current == entry.applied or IsOverridable(current)) then
		local target = desired or Cfg().DefaultGroup
		local done = true
		if current ~= target then
			local ok, result = pcall(adapter.Set, steam_id, ply, target)
			if not ok then
				GarryNet.Util.Warn(string.format("Couldn't set %s's usergroup to %s with %s: %s",
					steam_id, target, adapter.name, tostring(result)))
				done = false
			else
				done = result ~= false
				if done then
					GarryNet.Util.Log(string.format("Store: %s is now in usergroup %s", steam_id, target))
					if not quiet and ply and entry.applied and desired ~= entry.applied then
						GarryNet.NotifyPlayer(ply, "Your " .. entry.applied .. " usergroup from the store has ended.", nil)
					end
				end
			end
		end
		if done and entry.applied ~= desired then
			entry.applied = desired
			changed = true
		end
	end

	if #entry.grants == 0 and entry.applied == nil then
		Players[steam_id] = nil
		changed = true
	end
	if changed then R.Save() end
end

-- Weapons --------------------------------------------------------------------------------------

local function KnownWeapon(class)
	return weapons.GetStored(class) ~= nil or list.Get("Weapon")[class] ~= nil
end

local function HasWeaponGrant(entry, class)
	for _, g in ipairs(entry.grants) do
		if g.kind == "weapon" and g.class == class then return true end
	end
	return false
end

-- Store weapons are permanent: give them back on every spawn, after the gamemode's loadout.
local function GiveWeapons(ply)
	if not IsValid(ply) or not ply:Alive() then return end
	if isfunction(ply.isArrested) and ply:isArrested() then return end
	local entry = Players[ply:SteamID64()]
	if not entry then return end
	for _, g in ipairs(entry.grants) do
		if g.kind == "weapon" and not ply:HasWeapon(g.class) then ply:Give(g.class) end
	end
end

hook.Add("PlayerSpawn", "GarryNet.Rewards.Weapons", function(ply)
	timer.Simple(1, function() GiveWeapons(ply) end)
end)

-- Applying and revoking ------------------------------------------------------------------------

local function FormatDuration(seconds)
	local units = { { "day", 86400 }, { "hour", 3600 }, { "minute", 60 } }
	for _, unit in ipairs(units) do
		if seconds >= unit[2] then
			local n = math.Round(seconds / unit[2])
			return n .. " " .. unit[1] .. (n == 1 and "" or "s")
		end
	end
	return seconds .. " seconds"
end

-- A reward in words, for chat notices.
function R.Describe(reward)
	if reward.type == "usergroup" then
		if reward.duration_seconds then
			return reward.usergroup .. " usergroup for " .. FormatDuration(reward.duration_seconds)
		end
		return reward.usergroup .. " usergroup (permanent)"
	elseif reward.type == "darkrp_money" then
		if istable(DarkRP) and isfunction(DarkRP.formatMoney) then return DarkRP.formatMoney(reward.amount) end
		return "$" .. string.Comma(reward.amount)
	elseif reward.type == "weapon" then
		local stored = weapons.GetStored(reward.class)
		return (stored and stored.PrintName and stored.PrintName ~= "" and stored.PrintName or reward.class)
	end
	return tostring(reward.type)
end

-- Whether a fulfillment's rewards can all be applied to `ply` right now, checked before
-- anything is applied so a purchase is never half-delivered.
--   returns "ok", "wait" (try again later, e.g. DarkRP hasn't loaded the player's money yet),
--   or "invalid", reason
function R.Validate(rewards, ply)
	if not istable(rewards) or #rewards == 0 then return "invalid", "the purchase has no rewards" end
	local adapter = R.Adapter()

	for _, r in ipairs(rewards) do
		if not istable(r) then return "invalid", "malformed reward" end

		if r.type == "usergroup" then
			if not isstring(r.usergroup) or r.usergroup == "" then return "invalid", "usergroup reward without a usergroup" end
			if IsProtected(r.usergroup) then
				return "invalid", "refusing to grant the protected usergroup " .. r.usergroup .. " (GarryNet.Config.Store.ProtectedGroups)"
			end
			if not adapter.GroupExists(r.usergroup) then
				return "invalid", "usergroup " .. r.usergroup .. " doesn't exist in " .. adapter.name
			end
			if r.duration_seconds ~= nil and (not isnumber(r.duration_seconds) or r.duration_seconds <= 0) then
				return "invalid", "invalid usergroup duration"
			end
		elseif r.type == "darkrp_money" then
			if not isnumber(r.amount) or r.amount <= 0 then return "invalid", "invalid money amount" end
			if not istable(DarkRP) or not isfunction(ply.addMoney) then
				return "invalid", "darkrp_money needs DarkRP, but this server runs " .. ((GAMEMODE and GAMEMODE.Name) or engine.ActiveGamemode())
			end
			if ply:getDarkRPVar("money") == nil then return "wait" end
		elseif r.type == "weapon" then
			if not isstring(r.class) or r.class == "" then return "invalid", "weapon reward without a class" end
			if not KnownWeapon(r.class) then return "invalid", "unknown weapon class " .. r.class end
		else
			return "invalid", "unknown reward type " .. tostring(r.type)
		end
	end
	return "ok"
end

-- Apply a validated fulfillment to an online player. Lasting rewards are recorded first, so a
-- retry never grants them twice.
function R.Apply(fulfillment, ply)
	local steam_id = fulfillment.steam_id
	local entry = Entry(steam_id, true)
	local recorded = false
	for _, g in ipairs(entry.grants) do
		if g.fulfillment_id == fulfillment.id then recorded = true break end
	end

	local now = os.time()
	for _, r in ipairs(fulfillment.rewards) do
		if r.type == "usergroup" then
			if not recorded then
				table.insert(entry.grants, {
					fulfillment_id = fulfillment.id, kind = "usergroup", usergroup = r.usergroup,
					duration = r.duration_seconds, granted_at = now,
				})
			end
		elseif r.type == "weapon" then
			if not recorded then
				table.insert(entry.grants, { fulfillment_id = fulfillment.id, kind = "weapon", class = r.class, granted_at = now })
			end
			if ply:Alive() and not ply:HasWeapon(r.class) then ply:Give(r.class) end
		elseif r.type == "darkrp_money" then
			ply:addMoney(r.amount)
		end
	end

	R.Save()
	R.Reconcile(steam_id, true)
end

-- Take back a refunded or disputed purchase: its usergroups and weapons (money isn't clawed
-- back). Works for offline players too; their usergroup is corrected when they next join if
-- the admin mod can't change it while they're away. Safe to call more than once.
function R.Revoke(fulfillment)
	local steam_id = fulfillment.steam_id
	local entry = Players[steam_id]
	if not entry then return end

	local kept, weapons_removed = {}, {}
	for _, g in ipairs(entry.grants) do
		if g.fulfillment_id == fulfillment.id then
			if g.kind == "weapon" then weapons_removed[#weapons_removed + 1] = g.class end
		else
			kept[#kept + 1] = g
		end
	end
	entry.grants = kept

	local ply = Online(steam_id)
	if ply then
		for _, class in ipairs(weapons_removed) do
			if not HasWeaponGrant(entry, class) and ply:HasWeapon(class) then ply:StripWeapon(class) end
		end
	end

	R.Save()
	R.Reconcile(steam_id, true)
end

-- The player's lasting grants, for the garrynet_store_grants command.
function R.GetGrants(steam_id)
	local entry = Players[steam_id]
	return entry and entry.grants or {}, entry and entry.applied
end

-- Expiries and joins ---------------------------------------------------------------------------

-- Re-check every known player once a minute, so timed usergroups end on time.
timer.Create("GarryNet.Rewards.Reconcile", GarryNet.Config.Store.ReconcileSeconds, 0, function()
	local ids = table.GetKeys(Players)
	for _, steam_id in ipairs(ids) do
		R.Reconcile(steam_id)
	end
end)

-- On join (once the admin mod has set the player's group), re-apply a plain usergroup and end
-- any that ran out while they were away.
hook.Add("PlayerInitialSpawn", "GarryNet.Rewards.Join", function(ply)
	if ply:IsBot() then return end
	local steam_id = ply:SteamID64()
	timer.Simple(GarryNet.Config.Store.JoinDelay, function()
		if IsValid(ply) then R.Reconcile(steam_id) end
	end)
end)

Load()
