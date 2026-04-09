const VIDEO_URL = "https://youtu.be/4nVoLX2taFg";

describe("YouTube 자막 번역 흐름", () => {
  it("홈 화면이 렌더링되고 URL 입력 필드가 존재한다", async () => {
    const input = await $('input[placeholder="YouTube URL을 붙여넣으세요..."]');
    await input.waitForExist({ timeout: 10_000 });
    expect(await input.isDisplayed()).toBe(true);
  });

  it("YouTube URL 입력 후 플레이어 화면으로 이동한다", async () => {
    const input = await $('input[placeholder="YouTube URL을 붙여넣으세요..."]');
    await input.setValue(VIDEO_URL);
    await browser.keys("Enter");

    // hash history 라우팅: #/watch/4nVoLX2taFg
    await browser.waitUntil(
      async () => {
        const url = await browser.getUrl();
        return url.includes("watch/4nVoLX2taFg");
      },
      { timeout: 10_000, timeoutMsg: "플레이어 화면으로 이동하지 못했습니다" },
    );
  });

  it("자막 로딩 상태 또는 번역 결과가 표시된다", async () => {
    // 번역 준비 중... 또는 자막 텍스트가 나타날 때까지 대기
    const result = await browser.waitUntil(
      async () => {
        // 로딩 상태 확인
        const loading = await $("*=번역 준비 중");
        if (await loading.isExisting()) return "loading";

        // 번역 진행 상태 확인
        const translating = await $("*=번역 중");
        if (await translating.isExisting()) return "translating";

        // 에러 메시지 확인 (Claude CLI 없는 경우 등)
        const error = await $("p.text-red-200");
        if (await error.isExisting()) return "error";

        return false;
      },
      { timeout: 30_000, timeoutMsg: "자막 관련 UI가 표시되지 않았습니다" },
    );

    expect(["loading", "translating", "error"]).toContain(result);
  });

  it("뒤로가기 버튼으로 홈 화면에 복귀한다", async () => {
    const backBtn = await $('button[aria-label="뒤로가기"]');
    await backBtn.waitForExist({ timeout: 5_000 });
    await backBtn.click();

    const input = await $('input[placeholder="YouTube URL을 붙여넣으세요..."]');
    await input.waitForExist({ timeout: 5_000 });
    expect(await input.isDisplayed()).toBe(true);
  });
});
