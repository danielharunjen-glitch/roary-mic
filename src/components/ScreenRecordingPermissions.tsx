import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import {
  checkScreenRecordingPermission,
  requestScreenRecordingPermission,
} from "tauri-plugin-macos-permissions-api";

type PermissionState = "request" | "verify" | "granted";

interface Props {
  compact?: boolean;
}

const ScreenRecordingPermissions: React.FC<Props> = ({ compact = false }) => {
  const { t } = useTranslation();
  const [hasPermission, setHasPermission] = useState<boolean>(false);
  const [state, setState] = useState<PermissionState>("request");

  const isMacOS = type() === "macos";

  useEffect(() => {
    if (!isMacOS) return;
    let cancelled = false;
    checkScreenRecordingPermission().then((granted) => {
      if (cancelled) return;
      setHasPermission(granted);
      setState(granted ? "granted" : "request");
    });
    return () => {
      cancelled = true;
    };
  }, [isMacOS]);

  const handleClick = async () => {
    if (state === "request") {
      try {
        await requestScreenRecordingPermission();
      } catch (err) {
        console.error("requestScreenRecordingPermission failed:", err);
      }
      setState("verify");
    } else if (state === "verify") {
      const granted = await checkScreenRecordingPermission();
      setHasPermission(granted);
      setState(granted ? "granted" : "verify");
    }
  };

  if (!isMacOS || hasPermission) return null;

  const buttonText =
    state === "request"
      ? t("screenRecording.requestPermission")
      : t("screenRecording.openSettings");

  return (
    <div
      className={`rounded-lg border border-mid-gray ${
        compact ? "p-3" : "p-4"
      }`}
    >
      <div className="flex justify-between items-start gap-3">
        <div className="space-y-1">
          <p className="text-sm font-medium">
            {t("screenRecording.permissionsRequired")}
          </p>
          <p className="text-xs text-mid-gray">
            {t("screenRecording.permissionsDescription")}
          </p>
          {state === "verify" && (
            <p className="text-xs text-mid-gray">
              {t("screenRecording.relaunchHint")}
            </p>
          )}
        </div>
        <button
          onClick={handleClick}
          className="px-3 py-1.5 text-sm font-semibold bg-mid-gray/10 border border-mid-gray/80 hover:bg-logo-primary/10 hover:border-logo-primary rounded cursor-pointer whitespace-nowrap"
        >
          {buttonText}
        </button>
      </div>
    </div>
  );
};

export default ScreenRecordingPermissions;
