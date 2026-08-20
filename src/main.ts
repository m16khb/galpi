import "./styles.css"

import { TauriBackend } from "./adapters/tauri-backend"
import { AppView } from "./ui/app-view"
import { AppController } from "./ui/controller"

const root = document.querySelector<HTMLElement>("#app")
if (root === null) {
  throw new Error("Galpi 앱 루트를 찾지 못했습니다.")
}

const controller = new AppController(new TauriBackend(), new AppView(root))
await controller.start()
window.addEventListener("beforeunload", () => controller.stop(), { once: true })
