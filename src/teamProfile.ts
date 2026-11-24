import { computed, shallowRef, watch } from "vue";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";

const avatarSize = 128;
const maxPngBytes = 70000;
const maxInputBytes = 10 * 1024 * 1024;
const pngPrefix = "data:image/png;base64,";
export const teamAvatarAccept = "image/jpeg,image/png,image/webp";
export const teamAvatarSources = shallowRef<ReadonlyMap<string, string>>(new Map());
export const teamAvatarAvailable = shallowRef(false);
export const teamAvatarSaving = shallowRef(false);
export const teamAvatarOnline = computed(() => Boolean(teamProject.value && teamWorkspace.value && teamClient.value
  && teamSession.value && teamSession.value.user.id === teamUser.value && !teamAccessIssue.value
  && (teamReachable.value ?? navigator.onLine)));

let scope = 0;
let savedRevision = 0;
let readRequest: { controller: AbortController; promise: Promise<void> } | undefined;
let writeRequest: AbortController | undefined;

watch([teamProject, teamUser, teamWorkspace, teamClient, teamAccessIssue], () => {
  ++scope;
  readRequest?.controller.abort();
  writeRequest?.abort();
  readRequest = undefined;
  writeRequest = undefined;
  teamAvatarSources.value = new Map();
  teamAvatarAvailable.value = false;
  teamAvatarSaving.value = false;
}, { flush: "sync" });

export function avatarImageSource(base64: unknown): string | null {
  if (typeof base64 !== "string" || base64.length < 60 || base64.length > 4 * Math.ceil(maxPngBytes / 3)
    || base64.length % 4 !== 0 || !/^[A-Za-z0-9+/]+={0,2}$/.test(base64)) return null;
  try {
    const bytes = atob(base64);
    if (bytes.length > maxPngBytes || !bytes.startsWith("\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x80\0\0\0\x80\x08")
      || !bytes.endsWith("\0\0\0\0IEND\xaeB\x60\x82")) return null;
    return pngPrefix + base64;
  } catch {
    return null;
  }
}

export function refreshTeamAvatars(): Promise<void> {
  if (readRequest) return readRequest.promise;
  const client = teamAvatarOnline.value ? teamClient.value : null;
  if (!client) return Promise.resolve();
  const workspace = teamWorkspace.value;
  const currentScope = scope;
  const revision = savedRevision;
  const controller = new AbortController();
  const current = () => !controller.signal.aborted && scope === currentScope && savedRevision === revision;
  const promise = (async () => {
    const avatars = new Map<string, string>();
    const pageSize = 100;
    for (let offset = 0; ; offset += pageSize) {
      const result = await client.rpc("team_profile_avatars", { target_workspace: workspace })
        .order("user_id").range(offset, offset + pageSize - 1).abortSignal(controller.signal).retry(false);
      if (!current()) return;
      if (result.error) {
        if ([401, 403, 404].includes(result.status) || ["42501", "42883", "PGRST202"].includes(result.error.code)) {
          teamAvatarAvailable.value = false;
          teamAvatarSources.value = new Map();
        }
        return;
      }
      if (!Array.isArray(result.data)) return;
      for (const row of result.data) {
        const source = avatarImageSource(row?.png_base64);
        if (typeof row?.user_id === "string" && source) avatars.set(row.user_id, source);
      }
      if (result.data.length < pageSize) break;
    }
    teamAvatarSources.value = avatars;
    teamAvatarAvailable.value = true;
  })().catch(() => {}).finally(() => {
    if (readRequest?.controller === controller) readRequest = undefined;
  });
  readRequest = { controller, promise };
  return promise;
}

export async function saveTeamAvatar(base64: string | null): Promise<boolean> {
  const client = teamAvatarOnline.value ? teamClient.value : null;
  if (!client) throw new Error("Profil fotoğrafını kaydetmek için bağlantı gerekli.");
  if (writeRequest) return false;
  const source = avatarImageSource(base64);
  if (base64 !== null && !source) throw new Error("Hazırlanan profil fotoğrafı geçersiz.");
  const workspace = teamWorkspace.value;
  const user = teamUser.value;
  const currentScope = scope;
  const controller = new AbortController();
  writeRequest = controller;
  teamAvatarSaving.value = true;
  try {
    const result = await client.rpc("team_set_profile_avatar", { target_workspace: workspace, png_base64: base64 })
      .abortSignal(controller.signal).retry(false);
    if (controller.signal.aborted || scope !== currentScope) return false;
    if (result.error) throw new Error("Profil fotoğrafı kaydedilemedi. Bağlantıyı kontrol edip tekrar deneyin.");
    ++savedRevision;
    readRequest?.controller.abort();
    readRequest = undefined;
    const avatars = new Map(teamAvatarSources.value);
    if (source) avatars.set(user, source);
    else avatars.delete(user);
    teamAvatarSources.value = avatars;
    return true;
  } catch (error) {
    if (controller.signal.aborted || scope !== currentScope) return false;
    throw error;
  } finally {
    if (writeRequest === controller) {
      writeRequest = undefined;
      teamAvatarSaving.value = false;
    }
  }
}

async function readPhoto(blob: Blob) {
  if (typeof createImageBitmap === "function") {
    try { return await createImageBitmap(blob); } catch { /* Try the WebView image decoder. */ }
  }
  const url = URL.createObjectURL(blob);
  try {
    return await new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = reject;
      image.src = url;
    });
  } finally {
    URL.revokeObjectURL(url);
  }
}

export async function prepareTeamAvatar(file: File): Promise<string> {
  if (file.size > maxInputBytes) throw new Error("Profil fotoğrafı en fazla 10 MB olabilir.");
  const header = new Uint8Array(await file.slice(0, 12).arrayBuffer());
  const first = String.fromCharCode(...header);
  const mime = first.startsWith("\x89PNG\r\n\x1a\n") ? "image/png" : first.startsWith("\xff\xd8\xff") ? "image/jpeg"
    : first.startsWith("RIFF") && first.slice(8) === "WEBP" ? "image/webp" : null;
  if (!mime) throw new Error("JPG, PNG veya WebP biçiminde bir fotoğraf seçin.");

  const image = await readPhoto(file.slice(0, file.size, mime)).catch(() => {
    throw new Error("Bu fotoğraf açılamadı. Başka bir dosya seçin.");
  });
  try {
    const width = "naturalWidth" in image ? image.naturalWidth : image.width;
    const height = "naturalHeight" in image ? image.naturalHeight : image.height;
    if (!width || !height || width > 8192 || height > 8192 || width * height > 32_000_000) {
      throw new Error("Bu fotoğrafın boyutları çok büyük. Daha küçük bir dosya seçin.");
    }
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = avatarSize;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Fotoğraf hazırlanamadı.");
    const side = Math.min(width, height);
    context.drawImage(image, (width - side) / 2, (height - side) / 2, side, side, 0, 0, avatarSize, avatarSize);
    const base64 = canvas.toDataURL("image/png").slice(pngPrefix.length);
    if (!avatarImageSource(base64)) throw new Error("Fotoğraf hazırlanamadı. Başka bir dosya seçin.");
    return base64;
  } finally {
    if ("close" in image) image.close();
  }
}
