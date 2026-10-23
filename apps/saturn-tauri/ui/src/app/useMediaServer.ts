// F2: the media server must resolve its port and token BEFORE the editor
// stage mounts. Recovery or open-project flows used to reach the editor while
// this hook was still awaiting `get_media_server_port`, so previews built
// stream URLs with port 0 and silently fell back to unstreamable sources.
// `mediaServerReady` lets App hold the editor stage until init has settled.
import { useCallback, useEffect, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { buildMediaStreamUrl } from "../utils/mediaUrl";

export function useMediaServer() {
  const [mediaServerPort, setMediaServerPort] = useState(0);
  const [mediaServerToken, setMediaServerToken] = useState("");
  const [mediaServerReady, setMediaServerReady] = useState(false);

  useEffect(() => {
    let settled = 0;
    const done = () => {
      settled += 1;
      if (settled === 2) setMediaServerReady(true);
    };
    invoke<number>("get_media_server_port")
      .then((port) => {
        if (port > 0) setMediaServerPort(port);
      })
      .catch((error) => console.warn("Could not get media server port:", error))
      .finally(done);

    invoke<string>("get_media_server_token")
      .then((token) => {
        if (token) setMediaServerToken(token);
      })
      .catch((error) => console.warn("Could not get media server token:", error))
      .finally(done);
  }, []);

  const getMediaUrl = useCallback((filePath: string) => {
    const streamUrl = buildMediaStreamUrl(filePath, mediaServerPort, mediaServerToken);
    return streamUrl || convertFileSrc(filePath);
  }, [mediaServerPort, mediaServerToken]);

  return { mediaServerPort, mediaServerToken, mediaServerReady, getMediaUrl };
}
