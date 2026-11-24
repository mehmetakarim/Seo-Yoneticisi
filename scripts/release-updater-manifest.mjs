import { readFile, writeFile } from "node:fs/promises";
import { createHash, createPublicKey, verify } from "node:crypto";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

const stableTag = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const previewTag = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-preview\.[1-9]\d*$/;

function base64(value, label) {
  const bytes = Buffer.from(value ?? "", "base64");
  if (!bytes.length || bytes.toString("base64") !== value) throw new Error(`${label}: geçersiz Base64.`);
  return bytes;
}

function envelope(encoded, label) {
  const lines = base64(encoded, label).toString("utf8").trimEnd().split(/\r?\n/);
  if (!lines[0]?.startsWith("untrusted comment: ")) throw new Error(`${label}: Minisign zarfı geçersiz.`);
  return lines;
}

function verifyBundle(signature, bundle, publicKey, keyId, label) {
  const lines = envelope(signature, label);
  const record = base64(lines[1], label);
  const algorithm = record.subarray(0, 2).toString("ascii");
  const commentPrefix = "trusted comment: ";
  if (lines.length !== 4 || !lines[2].startsWith(commentPrefix) ||
      record.length !== 74 || !["Ed", "ED"].includes(algorithm) ||
      !record.subarray(2, 10).equals(keyId)) {
    throw new Error(`${label}: imza biçimi veya güncelleme anahtarı uyuşmuyor.`);
  }
  const fileSignature = record.subarray(10);
  const signedContent = algorithm === "ED" ? createHash("blake2b512").update(bundle).digest() : bundle;
  const trustedComment = Buffer.from(lines[2].slice(commentPrefix.length), "utf8");
  const signedComment = Buffer.concat([fileSignature, trustedComment]);
  const globalSignature = base64(lines[3], label);
  if (globalSignature.length !== 64 || !verify(null, signedContent, publicKey, fileSignature) ||
      !verify(null, signedComment, publicKey, globalSignature)) {
    throw new Error(`${label}: paket imzası doğrulanamadı.`);
  }
}

function oneAsset(assets, matches, label) {
  const files = assets.filter(({ name }) => typeof name === "string" && matches(name));
  if (files.length !== 1 || files[0].state !== "uploaded" || !Number.isSafeInteger(files[0].size) ||
      files[0].size <= 0 || /[\\/]/.test(files[0].name)) {
    throw new Error(`${label}: tek bir tamamlanmış yayın dosyası gerekli.`);
  }
  return files[0];
}

export async function buildUpdaterManifest({ repo, tag, release, publicKey, readAsset, preview = false }) {
  const versionParts = (preview ? previewTag : stableTag).exec(tag);
  if (!/^[a-z0-9_.-]+\/[a-z0-9_.-]+$/i.test(repo) || !versionParts) {
    throw new Error(preview ? "Geçerli bir GitHub deposu ve ön sürüm etiketi gerekli."
      : "Geçerli bir GitHub deposu ve kararlı sürüm etiketi gerekli.");
  }
  if (release?.tagName !== tag || release.isDraft !== true || release.isPrerelease !== preview ||
      !Array.isArray(release.assets) || typeof release.body !== "string" || !release.body.trim()) {
    throw new Error("Beklenen sürüm için notları olan bir GitHub yayın taslağı gerekli.");
  }

  const version = versionParts.slice(1).join(".");
  const msiSuffix = new RegExp(`_${version.replaceAll(".", "\\.")}_x64_[a-z0-9-]+\\.msi$`, "i");
  const msi = oneAsset(release.assets, (name) => msiSuffix.test(name), "Windows MSI");
  const nsis = oneAsset(release.assets, (name) => name.endsWith(`_${version}_x64-setup.exe`), "Windows NSIS");
  const mac = oneAsset(release.assets, (name) => name.endsWith("_universal.app.tar.gz"), "macOS güncelleme paketi");
  oneAsset(release.assets, (name) => name.endsWith(`_${version}_universal.dmg`), "macOS kurulum paketi");

  const keyLines = envelope(publicKey.trim(), "Tauri güncelleme anahtarı");
  const key = base64(keyLines[1], "Tauri güncelleme anahtarı");
  if (keyLines.length !== 2 || key.length !== 42 || key.subarray(0, 2).toString("ascii") !== "Ed") {
    throw new Error("Tauri güncelleme anahtarı geçersiz.");
  }
  const verifier = createPublicKey({
    // Ed25519 SubjectPublicKeyInfo header followed by the raw Minisign public key.
    key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), key.subarray(10)]),
    format: "der",
    type: "spki",
  });
  const assetUrl = (name) => `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(name)}`;
  const entry = async ({ name, size }) => {
    const asset = oneAsset(release.assets, (candidate) => candidate === `${name}.sig`, `${name} imzası`);
    const [bundle, content] = await Promise.all([readAsset(name), readAsset(asset.name)]);
    if (bundle.length !== size || content.length !== asset.size) throw new Error(`${name}: paket veya imza dosyası eksik ya da kesilmiş.`);
    const signature = content.toString("utf8").trim();
    verifyBundle(signature, bundle, verifier, key.subarray(2, 10), name);
    return { signature, url: assetUrl(name) };
  };
  const [macUpdate, windowsMsi, windowsNsis] = await Promise.all([entry(mac), entry(msi), entry(nsis)]);
  return {
    version,
    notes: release.body.trim(),
    platforms: {
      "darwin-aarch64": macUpdate,
      "darwin-x86_64": macUpdate,
      "darwin-aarch64-app": macUpdate,
      "darwin-x86_64-app": macUpdate,
      "windows-x86_64": windowsMsi,
      "windows-x86_64-msi": windowsMsi,
      "windows-x86_64-nsis": windowsNsis,
    },
  };
}

async function main() {
  const { values } = parseArgs({
    options: {
      repo: { type: "string" },
      tag: { type: "string" },
      release: { type: "string" },
      assets: { type: "string" },
      output: { type: "string" },
      preview: { type: "boolean", default: false },
    },
  });
  if (!values.repo || !values.tag || !values.release || !values.assets || !values.output) {
    throw new Error("Depo, etiket, yayın verisi, paket dizini ve çıktı yolu gerekli.");
  }
  const [release, config] = await Promise.all([
    readFile(values.release, "utf8").then(JSON.parse),
    readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8").then(JSON.parse),
  ]);
  const manifest = await buildUpdaterManifest({
    repo: values.repo,
    tag: values.tag,
    release,
    preview: values.preview,
    publicKey: config.plugins.updater.pubkey,
    readAsset: (name) => readFile(join(values.assets, name)),
  });
  await writeFile(values.output, `${JSON.stringify(manifest, null, 2)}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
