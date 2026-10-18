import { useCallback, useEffect, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { buildMediaStreamUrl } from "../utils/mediaUrl";

export function useMediaServer() {
  const [mediaServerPort, setMediaServerPort] = useState(0);
  const [mediaServerToken, setMediaServerToken] = useState("");

  useEffect(() => {
    invoke<number>("get_media_server_port")
      .then((port) => {
        if (port > 0) setMediaServerPort(port);
      })
      .catch((error) => console.warn("Could not get media server port:", error));

    invoke<string>("get_media_server_token")
      .then((token) => {
        if (token) setMediaServerToken(token);
      })
      .catch((error) => console.warn("Could not get media server token:", error));
  }, []);

  const getMediaUrl = useCallback((filePath: string) => {
    const streamUrl = buildMediaStreamUrl(filePath, mediaServerPort, mediaServerToken);
    return streamUrl || convertFileSrc(filePath);
  }, [mediaServerPort, mediaServerToken]);

  return { mediaServerPort, mediaServerToken, getMediaUrl };
}
