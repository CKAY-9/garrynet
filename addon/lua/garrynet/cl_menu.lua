-- The !gnet menu: dark-blue panel over a darkened, blurred background (docs/ADDON.md).
--
-- Not linked: text box for the server access key (+ "Link" button).
-- Linked:     buttons for "Unlink" and "Force interval update".

GarryNet.Menu = GarryNet.Menu or {}

GarryNet.Menu.Colors = {
	Background = Color(10, 20, 45, 245), -- dark blue panel
	Overlay = Color(0, 0, 0, 150), -- darkening behind the panel
	Accent = Color(59, 130, 246),
	Text = Color(235, 240, 255),
}

local Frame = nil

function GarryNet.Menu.IsOpen()
	return IsValid(Frame)
end

function GarryNet.Menu.Close()
	if IsValid(Frame) then Frame:Remove() end
	Frame = nil
end

-- Build and show the menu for the current GarryNet.ClientState.
function GarryNet.Menu.Open()
	-- TODO: full-screen panel that paints the blurred+darkened background (Derma_DrawBackgroundBlur
	-- or a blur material), a centered dark-blue panel, and the Not linked / Linked content.
	-- Escape or a close button should call GarryNet.Menu.Close().
end

-- Rebuild the content after a status change.
function GarryNet.Menu.Refresh()
	-- TODO
end

-- Show an action result (success/failure message) inside the menu.
function GarryNet.Menu.ShowResult(success, message)
	-- TODO
end
