// action に default_popup を置かないため、アイコンクリックは onClicked に届く。
// ポップアップ/サイドパネルはマップ・ファセット UI に狭いので、全画面タブで開く。
chrome.action.onClicked.addListener(() => {
  chrome.tabs.create({ url: "index.html" });
});
