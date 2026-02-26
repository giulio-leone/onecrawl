export { WebActionManager, webActionManager } from "./manager.js";
export type { WebSession, WebActionError } from "./manager.js";
export {
  handleNavigate,
  handleClick,
  handleType,
  handlePressKey,
  handleGetCookies,
  handleSetCookies,
  handleScreenshot,
  handleUpload,
  handleWait,
  handleExtract,
  handleEvaluate,
  handleListSessions,
  handleCloseSession,
  handleConnectCDP,
  handleHumanWarmup,
  handleHumanClick,
  handleHumanType,
} from "./routes.js";
