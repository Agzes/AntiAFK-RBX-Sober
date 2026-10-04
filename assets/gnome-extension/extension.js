import Gio from "gi://Gio";
import Clutter from "gi://Clutter";
import Shell from "gi://Shell";
import * as Main from "resource:///org/gnome/shell/ui/main.js";
import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";

const DBUS_IFACE = `
<node>
  <interface name="dev.agzes.antiafk.Backend">
    <method name="Probe">
      <arg type="b" direction="out"/>
    </method>
    <method name="ListWindows">
      <arg type="s" direction="out"/>
    </method>
    <method name="FocusWindow">
      <arg type="s" direction="in"/>
      <arg type="b" direction="out"/>
    </method>
    <method name="SetMinimized">
      <arg type="s" direction="in"/>
      <arg type="b" direction="in"/>
      <arg type="b" direction="out"/>
    </method>
    <method name="PointerPosition">
      <arg type="s" direction="out"/>
    </method>
    <method name="WarpPointer">
      <arg type="i" direction="in"/>
      <arg type="i" direction="in"/>
      <arg type="b" direction="out"/>
    </method>
    <method name="CaptureWindowToFile">
      <arg type="s" direction="in"/>
      <arg type="s" direction="in"/>
      <arg type="b" direction="out"/>
    </method>
  </interface>
</node>`;

export default class AntiAfkExtension extends Extension {
    enable() {
        this._dbus = Gio.DBusExportedObject.wrapJSObject(DBUS_IFACE, this);
        this._dbus.export(Gio.DBus.session, "/dev/agzes/antiafk");
        this._nameId = Gio.bus_own_name(
            Gio.BusType.SESSION,
            "dev.agzes.antiafk",
            Gio.BusNameOwnerFlags.NONE,
            () => {},
            () => {},
            () => {},
        );
    }

    disable() {
        if (this._nameId) {
            Gio.bus_unown_name(this._nameId);
            this._nameId = 0;
        }
        if (this._dbus) {
            this._dbus.unexport();
            this._dbus = null;
        }
    }

    Probe() {
        return true;
    }

    ListWindows() {
        const windows = global
            .get_window_actors()
            .map((actor) => actor.meta_window)
            .filter((window) => window && !window.is_override_redirect());
        return JSON.stringify(
            windows.map((window) => {
                const rect = window.get_frame_rect();
                return {
                    id: String(window.get_id()),
                    title: window.get_title() || "",
                    class: window.get_wm_class
                        ? window.get_wm_class() || ""
                        : "",
                    desktop: window.get_desktop_file
                        ? window.get_desktop_file() || ""
                        : "",
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    active: window === global.display.focus_window,
                };
            }),
        );
    }

    FocusWindow(id) {
        const window = this._findWindow(id);
        if (!window) return false;
        if (window.minimized) window.unminimize();
        Main.activateWindow(window);
        return true;
    }

    SetMinimized(id, minimized) {
        const window = this._findWindow(id);
        if (!window) return false;
        if (minimized) window.minimize();
        else if (window.minimized) window.unminimize();
        return true;
    }

    PointerPosition() {
        const pointer = global.get_pointer();
        return JSON.stringify([pointer[0], pointer[1]]);
    }

    WarpPointer(x, y) {
        try {
            const seat = Clutter.get_default_backend().get_default_seat();
            if (seat && typeof seat.warp_pointer === "function") {
                seat.warp_pointer(x, y);
                return true;
            }
        } catch (_e) {}

        try {
            if (
                global.stage &&
                typeof global.stage.warp_pointer === "function"
            ) {
                global.stage.warp_pointer(x, y);
                return true;
            }
        } catch (_e) {}

        return false;
    }

    CaptureWindowToFile(id, filename) {
        try {
            const window = this._findWindow(id);
            if (!window) return false;
            const rect = window.get_frame_rect();
            const file = Gio.File.new_for_path(filename);
            const stream = file.replace(
                null,
                false,
                Gio.FileCreateFlags.REPLACE_DESTINATION,
                null,
            );

            let screenshot = new Shell.Screenshot();
            screenshot.screenshot_area(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                stream,
                (_obj, res) => {
                    try {
                        screenshot.screenshot_area_finish(res);
                    } catch (_err) {}
                    try {
                        stream.close(null);
                    } catch (_err) {}
                },
            );
            return true;
        } catch (e) {
            log(`[AntiAFK-RBX-Sober] CaptureWindowToFile error: ${e}`);
            return false;
        }
    }

    _findWindow(id) {
        return global
            .get_window_actors()
            .map((actor) => actor.meta_window)
            .find((window) => window && String(window.get_id()) === String(id));
    }
}
