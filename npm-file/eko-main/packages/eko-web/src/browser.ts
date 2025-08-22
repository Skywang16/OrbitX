import html2canvas from "html2canvas";
import { AgentContext, BaseBrowserLabelsAgent } from "@eko-ai/eko";

export default class BrowserAgent extends BaseBrowserLabelsAgent {
  protected async screenshot(
    agentContext: AgentContext
  ): Promise<{ imageBase64: string; imageType: "image/jpeg" | "image/png" }> {
    const [width, height] = this.size();
    const scrollX = window.scrollX || window.pageXOffset;
    const scrollY = window.scrollY || window.pageYOffset;
    const canvas = await html2canvas(document.documentElement || document.body, {
      width,
      height,
      windowWidth: width,
      windowHeight: height,
      x: scrollX,
      y: scrollY,
      scrollX: -scrollX,
      scrollY: -scrollY,
      useCORS: true,
      foreignObjectRendering: true,
      // backgroundColor: 'white',
      // scale: window.devicePixelRatio || 1,
    });
    let dataUrl = canvas.toDataURL("image/jpeg");
    let data = dataUrl.substring(dataUrl.indexOf("base64,") + 7);
    return {
      imageBase64: data,
      imageType: "image/jpeg",
    };
  }

  protected async navigate_to(
    agentContext: AgentContext,
    url: string
  ): Promise<{ url: string; title?: string }> {
    let idx = location.href.indexOf("/", 10);
    let baseUrl = idx > -1 ? location.href.substring(0, idx) : location.href;
    if (url.startsWith("/")) {
      history.pushState(null, "", url);
    } else if (url.startsWith(baseUrl)) {
      history.pushState(null, "", url.substring(baseUrl.length));
    } else {
      throw new Error(
        "Unable to access other websites, can only access other subpages within the current website: " +
          baseUrl
      );
    }
    window.dispatchEvent(new PopStateEvent("popstate"));
    await this.sleep(200);
    return {
      url: location.href,
      title: document.title,
    };
  }

  protected async execute_script(
    agentContext: AgentContext,
    func: (...args: any[]) => void,
    args: any[]
  ): Promise<any> {
    return func(args[0]);
  }

  private size(): [number, number] {
    return [
      window.innerWidth ||
        document.documentElement.clientWidth ||
        (document.documentElement || document.body).clientWidth,
      window.innerHeight ||
        document.documentElement.clientHeight ||
        (document.documentElement || document.body).clientHeight,
    ];
  }

  private sleep(time: number): Promise<void> {
    return new Promise((resolve) => setTimeout(() => resolve(), time));
  }

  protected async get_all_tabs(
    agentContext: AgentContext
  ): Promise<Array<{ tabId: number; url: string; title: string }>> {
    return [
      {
        tabId: 0,
        url: location.href,
        title: document.title,
      },
    ];
  }

  protected async switch_tab(
    agentContext: AgentContext,
    tabId: number
  ): Promise<{ tabId: number; url: string; title: string }> {
    return await this.get_all_tabs(agentContext)[0];
  }
}

export { BrowserAgent };