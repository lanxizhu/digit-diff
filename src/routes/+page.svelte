<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onDestroy, onMount } from "svelte";
  import * as Tesseract from "tesseract.js";

  type CompareStatus = "idle" | "missing" | "match" | "mismatch";
  type InputSide = "left" | "right";
  type OcrStatus = "idle" | "reading" | "error";

  let challenger = $state("");
  let defender = $state("");
  let leftOcrStatus = $state<OcrStatus>("idle");
  let rightOcrStatus = $state<OcrStatus>("idle");
  let leftImagePreview = $state("");
  let rightImagePreview = $state("");
  let activePreview = $state("");
  let isPinned = $state(false);
  let isPinPending = $state(false);
  let leftTextArea: HTMLTextAreaElement;
  let rightTextArea: HTMLTextAreaElement;
  let leftFileInput: HTMLInputElement;
  let rightFileInput: HTMLInputElement;

  function normalizeText(value: string) {
    return value.replace(/[\s,_，]/g, "");
  }

  const leftText = $derived(normalizeText(challenger));
  const rightText = $derived(normalizeText(defender));

  const compareStatus = $derived.by<CompareStatus>(() => {
    if (!leftText && !rightText) return "idle";
    if (!leftText || !rightText) return "missing";

    return leftText === rightText ? "match" : "mismatch";
  });

  const resultTitle = $derived.by(() => {
    switch (compareStatus) {
      case "match":
        return "数字相等";
      case "mismatch":
        return "数字不相等";
      case "missing":
        return "等待另一方数字";
      default:
        return "粘贴两方数字";
    }
  });

  const roundHint = $derived.by(() => {
    switch (compareStatus) {
      case "match":
        return "双方数字一致，本局通过。";
      case "mismatch":
        return "双方数字不同，请重新核对。";
      case "missing":
        return "请补齐两侧数字。";
      default:
        return "粘贴两组数字后自动判断。";
    }
  });

  const resultClass = $derived(`result ${compareStatus}`);

  async function clearAll() {
    challenger = "";
    defender = "";
    leftOcrStatus = "idle";
    rightOcrStatus = "idle";
    clearImagePreview("left", false);
    clearImagePreview("right", false);

    setWindowSize("small");
  }

  async function togglePinned() {
    const nextPinned = !isPinned;

    isPinPending = true;

    try {
      await getCurrentWindow().setAlwaysOnTop(nextPinned);
      isPinned = nextPinned;
    } catch (error) {
      console.error("Failed to toggle always-on-top:", error);
    } finally {
      isPinPending = false;
    }
  }

  function getOcrStatus(side: InputSide) {
    return side === "left" ? leftOcrStatus : rightOcrStatus;
  }

  function setOcrStatus(side: InputSide, status: OcrStatus) {
    if (side === "left") {
      leftOcrStatus = status;
    } else {
      rightOcrStatus = status;
    }
  }

  function setSideValue(side: InputSide, value: string) {
    if (side === "left") {
      challenger = value;
    } else {
      defender = value;
    }
  }

  function getImagePreview(side: InputSide) {
    return side === "left" ? leftImagePreview : rightImagePreview;
  }

  function syncWindowSizeToImagePreviews() {
    if (leftImagePreview && rightImagePreview) {
      setWindowSize("large");
    } else if (leftImagePreview || rightImagePreview) {
      setWindowSize("medium");
    } else {
      setWindowSize("small");
    }
  }

  function clearImagePreview(side: InputSide, resize = true) {
    const preview = getImagePreview(side);

    if (preview) {
      if (activePreview === preview) {
        activePreview = "";
      }

      URL.revokeObjectURL(preview);
    }

    if (side === "left") {
      leftImagePreview = "";
    } else {
      rightImagePreview = "";
    }

    if (resize) {
      syncWindowSizeToImagePreviews();
    }
  }

  function clearSide(side: InputSide) {
    setSideValue(side, "");
    setOcrStatus(side, "idle");
    clearImagePreview(side);
  }

  function setImagePreview(side: InputSide, image: File | Blob) {
    clearImagePreview(side, false);

    const preview = URL.createObjectURL(image);

    if (side === "left") {
      leftImagePreview = preview;
    } else {
      rightImagePreview = preview;
    }

    syncWindowSizeToImagePreviews();
  }

  async function setWindowSize(size: "small" | "medium" | "large") {
    const appwindow = getCurrentWindow();
    const info = await appwindow.innerSize()

    const { width } = info;

    const HEIGHT_BASE = __APP_PLATFORM__ === "windows" ? 515 * 1 : 547 * 2;

    const imageHeight = (120 + 8) * (__APP_PLATFORM__ === "windows" ? 1 : 2);

    let height: number;
    
    switch (size) {
      case "small":
        height = HEIGHT_BASE;
        break;
      case "medium":
        height = HEIGHT_BASE + imageHeight;
        break;
      case "large":
        height = HEIGHT_BASE + imageHeight * 2;
        break;
    }

    invoke("animate_window_size", { width, height });
  }

  let focusHandled = $state(false);

  function focusPrimaryTextArea() {
    requestAnimationFrame(() => {
      const active = document.activeElement as HTMLElement;
      if (!focusHandled) {
        focusHandled = true;
        setTimeout(() => {
      if (challenger && !defender) {
        rightTextArea?.focus();
      } else  {
        leftTextArea?.focus();
          }
        }, 50);
      }
    });
  }

  async function recognizeImage(side: InputSide, image: File | Blob) {
    setImagePreview(side, image);
    setOcrStatus(side, "reading");

    try {
      const worker = await Tesseract.createWorker(["eng", "chi_sim"]);

      try {
        // await worker.setParameters({
        //   // tessedit_char_whitelist: "0123456789.,+-_ ",
        //   tessedit_pageseg_mode: Tesseract.PSM.SINGLE_LINE
        // });

        const {
          data: { text }
        } = await worker.recognize(image);
        const res = normalizeText(text);

        if (!res) {
          setOcrStatus(side, "error");
          return;
        }

        setSideValue(side, res);
        setOcrStatus(side, "idle");
      } finally {
        await worker.terminate();
      }
    } catch {
      setOcrStatus(side, "error");
    }
  }

  function handleImageSelect(side: InputSide, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];

    if (!file) return;

    void recognizeImage(side, file);
    input.value = "";
  }

  function handlePaste(side: InputSide, event: ClipboardEvent) {
    const imageFile = Array.from(event.clipboardData?.files ?? []).find((file) =>
      file.type.startsWith("image/")
    );

    if (!imageFile) {
      clearImagePreview(side);
      return;
    }

    event.preventDefault();
    void recognizeImage(side, imageFile);
  }

  // 1. 处理窗口失去焦点
  const handleWindowBlur = () => {
    // 【关键修复】窗口失焦时，主动取消当前元素的聚焦状态
    // 这样 Tauri 再次获焦时，就不会自动强行恢复之前的焦点，从而彻底消除闪烁
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
    focusHandled = false;
  };

  function handleTextareaFoucs(event: Event) {
    // 解决 macOS 上的聚焦问题
    event.stopPropagation();
    const target = event.currentTarget as HTMLTextAreaElement;
    setTimeout(() => target.select(), 10);
    focusHandled = true;
  }

  onDestroy(() => {
    clearImagePreview("left");
    clearImagePreview("right");
    setWindowSize("small");
  });

  onMount(() => {
    const currentWindow = getCurrentWindow();

    void currentWindow
      .isAlwaysOnTop()
      .then((alwaysOnTop) => {
        isPinned = alwaysOnTop;
      })
      .catch((error) => {
        console.error("Failed to read always-on-top state:", error);
      });

    if (__APP_PLATFORM__ === "windows") {
      document.getElementById("drag-region")?.remove();
      document.querySelector(".app-shell")?.classList.add("windows-fix");
    }
  });
</script>

<svelte:window
  onfocus={focusPrimaryTextArea}
  onblur={handleWindowBlur}
  onkeydown={(event) => {
    if (event.key === "Escape") {
      activePreview = "";
    }
  }}
/>

<svelte:head>
  <title>Digit Diff</title>
</svelte:head>

<main class="app-shell">
  <section class="compare-panel" aria-labelledby="page-title">
    <div class="panel-header">
      <div class="title-row">
        <h1 id="page-title">数字核对</h1>
        <button
          class:active={isPinned}
          class="pin-button"
          type="button"
          disabled={isPinPending}
          aria-pressed={isPinned}
          onclick={togglePinned}
        >
          {isPinned ? "取消置顶" : "置顶"}
        </button>
      </div>

      <button class="ghost-button" type="button" onclick={clearAll}>清空</button>
    </div>

    <div class="inputs-grid">
      <div class="number-field">
        <div class="field-header">
          <label for="left-number">左侧数字</label>
          <div class="field-actions">
            <button class="image-button" type="button" onclick={() => leftFileInput.click()}>
              选择图片
            </button>
            <button class="image-button" type="button" onclick={() => clearSide("left")}>
              清除
            </button>
          </div>
        </div>
        <textarea
          bind:this={leftTextArea}
          id="left-number"
          bind:value={challenger}
          placeholder="粘贴数字或图片"
          spellcheck="false"
          onpaste={(event) => handlePaste("left", event)}
          onfocus={handleTextareaFoucs}
        ></textarea>
        <input
          bind:this={leftFileInput}
          class="file-input"
          type="file"
          accept="image/*"
          onchange={(event) => handleImageSelect("left", event)}
        />
        {#if getImagePreview("left")}
          <button
            class="preview-button"
            type="button"
            aria-label="查看左侧大图"
            onclick={() => (activePreview = getImagePreview("left"))}
          >
            <img class="image-preview" src={getImagePreview("left")} alt="左侧图片预览" />
          </button>
        {/if}
        {#if getOcrStatus("left") === "reading"}
          <p class="ocr-note">识别中...</p>
        {:else if getOcrStatus("left") === "error"}
          <p class="ocr-note error">未识别到数字</p>
        {/if}
      </div>

      <div class="number-field">
        <div class="field-header">
          <label for="right-number">右侧数字</label>
          <div class="field-actions">
            <button class="image-button" type="button" onclick={() => rightFileInput.click()}>
              选择图片
            </button>
            <button class="image-button" type="button" onclick={() => clearSide("right")}>
              清除
            </button>
          </div>
        </div>
        <textarea
          bind:this={rightTextArea}
          id="right-number"
          bind:value={defender}
          placeholder="粘贴数字或图片"
          spellcheck="false"
          onpaste={(event) => handlePaste("right", event)}
          onfocus={handleTextareaFoucs}
        ></textarea>
        <input
          bind:this={rightFileInput}
          class="file-input"
          type="file"
          accept="image/*"
          onchange={(event) => handleImageSelect("right", event)}
        />
        {#if getImagePreview("right")}
          <button
            class="preview-button"
            type="button"
            aria-label="查看右侧大图"
            onclick={() => (activePreview = getImagePreview("right"))}
          >
            <img class="image-preview" src={getImagePreview("right")} alt="右侧图片预览" />
          </button>
        {/if}
        {#if getOcrStatus("right") === "reading"}
          <p class="ocr-note">识别中...</p>
        {:else if getOcrStatus("right") === "error"}
          <p class="ocr-note error">未识别到数字</p>
        {/if}
      </div>
    </div>

    <section class={resultClass} aria-live="polite">
      <span class="result-badge">
        {#if compareStatus === "match"}
          =
        {:else if compareStatus === "mismatch"}
          ≠
        {:else}
          ?
        {/if}
      </span>
      <div>
        <h2>{resultTitle}</h2>
        <p>{roundHint}</p>
      </div>
    </section>
  </section>
</main>

{#if activePreview}
  <div class="image-modal" role="dialog" aria-modal="true" tabindex="-1">
    <button class="modal-backdrop" type="button" aria-label="关闭大图" onclick={() => (activePreview = "")}></button>
    <button class="modal-close" type="button" aria-label="关闭大图" onclick={() => (activePreview = "")}>
      关闭
    </button>
    <div class="modal-image-wrap">
      <img src={activePreview} alt="图片大图" />
    </div>
  </div>
{/if}
