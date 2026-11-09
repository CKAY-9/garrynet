-- Shared helpers.

GarryNet.Util = GarryNet.Util or {}

function GarryNet.Util.Log(msg)
	MsgC(Color(59, 130, 246), "[GarryNet] ", color_white, tostring(msg), "\n")
end

function GarryNet.Util.Warn(msg)
	MsgC(Color(59, 130, 246), "[GarryNet] ", Color(250, 204, 21), tostring(msg), "\n")
end

-- True if the string looks like a GarryNet access key. Does NOT verify it with the backend.
function GarryNet.Util.IsValidAccessKeyFormat(key)
	return isstring(key)
		and #key == GarryNet.Config.AccessKeyLength
		and string.match(key, GarryNet.Config.AccessKeyPattern) ~= nil
end
