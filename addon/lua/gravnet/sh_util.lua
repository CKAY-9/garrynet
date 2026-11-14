-- Shared helpers.

GravNet.Util = GravNet.Util or {}

function GravNet.Util.Log(msg)
	MsgC(Color(59, 130, 246), "[GravNet] ", color_white, tostring(msg), "\n")
end

function GravNet.Util.Warn(msg)
	MsgC(Color(59, 130, 246), "[GravNet] ", Color(250, 204, 21), tostring(msg), "\n")
end

-- True if the string looks like a GravNet access key. Does NOT verify it with the backend.
function GravNet.Util.IsValidAccessKeyFormat(key)
	return isstring(key)
		and #key == GravNet.Config.AccessKeyLength
		and string.match(key, GravNet.Config.AccessKeyPattern) ~= nil
end
