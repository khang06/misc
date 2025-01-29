require("ui.uieditor.menus.craft.emblemeditor.emblemeditorog")

local cmds_str = "COMMANDS GO HERE!!!!"
local cmds = {}
for x in string.gmatch(cmds_str, "[^;]+") do
    cmds[#cmds + 1] = x
end
local cmd_idx = 1

EmblemEditor_og = LUI.createMenu.EmblemEditor
LUI.createMenu.EmblemEditor = function(_controller)
    cmd_idx = 1

    local _menu = EmblemEditor_og(_controller)
    _menu:AddButtonCallbackFunction(_menu, _controller, Enum.LUIButton.LUI_KEY_LSTICK_PRESSED, "I", function (element, menu, controller, model)
        menu:addElement(LUI.UITimer.newElementTimer(0, false, function ()
            if cmd_idx <= #cmds then
                Engine.ExecNow(controller, cmds[cmd_idx])
                cmd_idx = cmd_idx + 1
            end
        end))
        return true
    end, function (element, menu, controller)
        CoD.Menu.SetButtonLabel(menu, Enum.LUIButton.LUI_KEY_LSTICK_PRESSED, "")
        return true
    end, false)

    return _menu
end
