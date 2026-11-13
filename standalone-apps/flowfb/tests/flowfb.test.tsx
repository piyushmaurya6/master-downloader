import { describe, it, expect } from "vitest";
import {
  classifyFacebookUrlClient,
  formatBytes,
  formatDuration,
  formatEta,
  formatCompactNumber,
} from "../src/lib/utils";
import { useUIStore } from "../src/stores/useUIStore";

describe("Facebook URL Client Classifier", () => {
  it("recognizes and canonicalizes Facebook Reel URLs with tracking params", () => {
    const res = classifyFacebookUrlClient(
      "https://www.facebook.com/reel/123456789012345/?s=single_unit&__cft__[0]=AZV123"
    );
    expect(res.valid).toBe(true);
    expect(res.badge).toBe("REEL");
    expect(res.mediaId).toBe("123456789012345");
    expect(res.canonicalUrl).toBe(
      "https://www.facebook.com/reel/123456789012345"
    );
  });

  it("recognizes Facebook Watch URLs", () => {
    const res = classifyFacebookUrlClient(
      "https://m.facebook.com/watch/?v=987654321098765&rdid=xyz999"
    );
    expect(res.valid).toBe(true);
    expect(res.badge).toBe("WATCH");
    expect(res.mediaId).toBe("987654321098765");
    expect(res.canonicalUrl).toBe(
      "https://www.facebook.com/watch/?v=987654321098765"
    );
  });

  it("recognizes fb.watch shortlinks", () => {
    const res = classifyFacebookUrlClient("https://fb.watch/xYz123AbC/?mibextid=123");
    expect(res.valid).toBe(true);
    expect(res.badge).toBe("FB.WATCH");
    expect(res.mediaId).toBe("xYz123AbC");
    expect(res.canonicalUrl).toBe("https://fb.watch/xYz123AbC/");
  });

  it("recognizes modern Facebook /share/{v|r|p}/{token} URLs", () => {
    const reelShare = classifyFacebookUrlClient(
      "https://www.facebook.com/share/r/1AbCdEfGh/?mibextid=WC7FNe"
    );
    expect(reelShare.valid).toBe(true);
    expect(reelShare.badge).toBe("REEL");
    expect(reelShare.mediaId).toBe("1AbCdEfGh");

    const videoShare = classifyFacebookUrlClient(
      "https://www.facebook.com/share/v/9ZyXwVuTs/"
    );
    expect(videoShare.valid).toBe(true);
    expect(videoShare.badge).toBe("VIDEO");
    expect(videoShare.mediaId).toBe("9ZyXwVuTs");
  });

  it("recognizes Page video and story.php / permalink.php links", () => {
    const pageVideo = classifyFacebookUrlClient(
      "https://www.facebook.com/nasa/videos/10159988776655443/"
    );
    expect(pageVideo.valid).toBe(true);
    expect(pageVideo.badge).toBe("VIDEO");
    expect(pageVideo.mediaId).toBe("10159988776655443");

    const storyPhp = classifyFacebookUrlClient(
      "https://m.facebook.com/story.php?story_fbid=pfbid02AbC123&id=1000445566"
    );
    expect(storyPhp.valid).toBe(true);
    expect(storyPhp.badge).toBe("POST");
    expect(storyPhp.mediaId).toBe("pfbid02AbC123");
  });

  it("rejects non-Facebook domains", () => {
    const res = classifyFacebookUrlClient(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );
    expect(res.valid).toBe(false);
    expect(res.badge).toBeNull();
  });
});

describe("Formatting Utilities", () => {
  it("formats byte sizes cleanly", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1048576 * 12.5)).toBe("12.5 MB");
  });

  it("formats durations and ETAs accurately", () => {
    expect(formatDuration(65)).toBe("01:05");
    expect(formatDuration(3665)).toBe("1:01:05");
    expect(formatEta(125)).toBe("02:05");
  });

  it("formats compact view/like counts", () => {
    expect(formatCompactNumber(950)).toBe("950");
    expect(formatCompactNumber(14500)).toBe("14.5K");
    expect(formatCompactNumber(2400000)).toBe("2.4M");
  });
});

describe("UI Store State", () => {
  it("switches active tabs and manages omnibar input", () => {
    const store = useUIStore.getState();
    store.setActiveTab("history");
    expect(useUIStore.getState().activeTab).toBe("history");

    store.setOmnibarInput("https://www.facebook.com/reel/123456789012345");
    expect(useUIStore.getState().omnibarInput).toBe(
      "https://www.facebook.com/reel/123456789012345"
    );
  });
});
