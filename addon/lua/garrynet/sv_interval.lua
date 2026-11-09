-- The GNet interval: every minute, sync server and player information to GarryNet.
-- Requests go to /gnet/server/{SERVER_ID}/[route] (see GarryNet.HTTP.ServerRequest).
-- The first successful sync after linking is the server's first heartbeat: the backend fills in
-- gamemode and map from it (docs/SERVERS.md).

GarryNet.Interval = GarryNet.Interval or {}

local TIMER = "GarryNet.Interval"

-- Collect what the backend needs for a sync: the server's gamemode/map and the connected players
-- (Steam ID 64, name, playtime, last logon, gamemode attributes ...).
-- Returns a table that will be sent as the JSON body.
function GarryNet.Interval.BuildPayload()
	-- TODO: decide the payload shape together with the backend routes.
	return {}
end

-- Run one sync immediately. Also used by the "force interval update" menu option.
--   callback: function(success: boolean, message: string)
function GarryNet.Interval.Run(callback)
	-- TODO: send BuildPayload() via GarryNet.HTTP.ServerRequest, then fetch pending work
	-- (unrealized store requests, config changes, ...) and hand it to handlers.
	if callback then callback(false, "Not implemented") end
end

-- Start the repeating timer (no-op if not linked or already running).
function GarryNet.Interval.Start()
	if not GarryNet.Storage.IsLinked() then return end

	timer.Create(TIMER, GarryNet.Config.IntervalSeconds, 0, function()
		GarryNet.Interval.Run()
	end)
end

function GarryNet.Interval.Stop()
	timer.Remove(TIMER)
end

hook.Add("InitPostEntity", "GarryNet.Interval.Start", GarryNet.Interval.Start)
