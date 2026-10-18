import test from "node:test";
import assert from "node:assert/strict";
import { filterMediaItems } from "./mediaFilter.ts";
import type { MediaAsset } from "./projectTypes.ts";

const mockMedia: MediaAsset[] = [
  { id: 1, name: "A-Roll Interview.mp4", path: "/media/A-Roll Interview.mp4", kind: "video", codec: "h264", tags: ["interview", "primary", "4k"], bin_id: 10 },
  { id: 2, name: "B-Roll City Sunset.mov", path: "/media/B-Roll City Sunset.mov", kind: "video", codec: "prores", tags: ["city", "drone"], bin_id: 10 },
  { id: 3, name: "Background Music.wav", path: "/audio/Background Music.wav", kind: "audio", codec: "pcm", tags: ["music", "upbeat"], bin_id: 20 },
  { id: 4, name: "Voiceover Narration.mp3", path: "/audio/Voiceover Narration.mp3", kind: "audio", codec: "mp3", tags: ["voice"], bin_id: null },
  { id: 5, name: "Lower Third Graphic.png", path: "/graphics/Lower Third Graphic.png", kind: "image", codec: "png", tags: ["title"], bin_id: null },
];

test("filterMediaItems returns all items when search is empty and binId is null", () => {
  const result = filterMediaItems(mockMedia, "", null);
  assert.equal(result.length, 5);
});

test("filterMediaItems filters by bin ID correctly", () => {
  const bin10Items = filterMediaItems(mockMedia, "", 10);
  assert.equal(bin10Items.length, 2);
  assert.equal(bin10Items[0].name, "A-Roll Interview.mp4");
  assert.equal(bin10Items[1].name, "B-Roll City Sunset.mov");

  const bin20Items = filterMediaItems(mockMedia, "", 20);
  assert.equal(bin20Items.length, 1);
  assert.equal(bin20Items[0].name, "Background Music.wav");

  const emptyBin = filterMediaItems(mockMedia, "", 999);
  assert.equal(emptyBin.length, 0);
});

test("filterMediaItems searches case-insensitively across name and path", () => {
  const interviewSearch = filterMediaItems(mockMedia, "interview", null);
  assert.equal(interviewSearch.length, 1);
  assert.equal(interviewSearch[0].id, 1);

  const audioPathSearch = filterMediaItems(mockMedia, "/audio/", null);
  assert.equal(audioPathSearch.length, 2);
});

test("filterMediaItems searches across media codec and tags", () => {
  const proresSearch = filterMediaItems(mockMedia, "prores", null);
  assert.equal(proresSearch.length, 1);
  assert.equal(proresSearch[0].name, "B-Roll City Sunset.mov");

  const droneTagSearch = filterMediaItems(mockMedia, "drone", null);
  assert.equal(droneTagSearch.length, 1);
  assert.equal(droneTagSearch[0].id, 2);

  const upbeatSearch = filterMediaItems(mockMedia, "UPBEAT", null);
  assert.equal(upbeatSearch.length, 1);
  assert.equal(upbeatSearch[0].id, 3);
});

test("filterMediaItems combines bin filtering and search query", () => {
  // In bin 10, search for "city" matches B-Roll but not A-Roll
  const cityInBin10 = filterMediaItems(mockMedia, "city", 10);
  assert.equal(cityInBin10.length, 1);
  assert.equal(cityInBin10[0].id, 2);

  // In bin 20, search for "city" matches nothing
  const cityInBin20 = filterMediaItems(mockMedia, "city", 20);
  assert.equal(cityInBin20.length, 0);
});

test("filterMediaItems handles empty or invalid inputs gracefully", () => {
  assert.deepEqual(filterMediaItems(undefined as any, "", null), []);
  assert.deepEqual(filterMediaItems([], "test", null), []);
  assert.deepEqual(filterMediaItems([{ id: 1, name: "clip", path: "", kind: "video" }] as any, "  ", null).length, 1);
});
