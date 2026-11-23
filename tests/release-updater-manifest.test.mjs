import assert from "node:assert/strict";
import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { test } from "node:test";
import { buildUpdaterManifest } from "../scripts/release-updater-manifest.mjs";

const repo = "mehmetakarim/Seo-Yoneticisi";
const tag = "v0.22.0";
const msi = "SEO.Yoneticisi_0.22.0_x64_en-US.msi";
const nsis = "SEO.Yoneticisi_0.22.0_x64-setup.exe";
const mac = "SEO.Yoneticisi_universal.app.tar.gz";
const dmg = "SEO.Yoneticisi_0.22.0_universal.dmg";
const keyId = Buffer.alloc(8, 7);
const trustedComment = "timestamp:42\tfile:release";

function signingKey() {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const rawKey = publicKey.export({ format: "der", type: "spki" }).subarray(-32);
  const record = Buffer.concat([Buffer.from("Ed"), keyId, rawKey]);
  const envelope = `untrusted comment: minisign public key\n${record.toString("base64")}\n`;
  return { privateKey, publicKey: Buffer.from(envelope).toString("base64") };
}

function makeSignature(bytes, privateKey, algorithm) {
  const message = algorithm === "Ed" ? bytes : createHash("blake2b512").update(bytes).digest();
  const signature = sign(null, message, privateKey);
  const record = Buffer.concat([Buffer.from(algorithm), keyId, signature]);
  const signedComment = Buffer.concat([signature, Buffer.from(trustedComment)]);
  const globalSignature = sign(null, signedComment, privateKey);
  const envelope = [
    "untrusted comment: minisign signature",
    record.toString("base64"),
    `trusted comment: ${trustedComment}`,
    globalSignature.toString("base64"),
    "",
  ].join("\n");
  return Buffer.from(Buffer.from(envelope).toString("base64"));
}

function fixture() {
  const { privateKey, publicKey } = signingKey();
  const files = new Map();
  for (const [name, algorithm] of [[msi, "ED"], [nsis, "ED"], [mac, "Ed"]]) {
    const bytes = Buffer.from(`Fixture for ${name}`);
    files.set(name, bytes);
    files.set(`${name}.sig`, makeSignature(bytes, privateKey, algorithm));
  }
  files.set(dmg, Buffer.from("Disk image fixture"));
  const assets = Array.from(files, ([name, bytes]) => ({ name, size: bytes.length, state: "uploaded" }));
  const release = { tagName: tag, isDraft: true, isPrerelease: false, body: "Sürüm notları", assets };
  return { repo, tag, release, publicKey, files, readAsset: async (name) => files.get(name) };
}

test("builds the seven updater entries from three independently verified packages", async () => {
  const input = fixture();
  const manifest = await buildUpdaterManifest(input);
  const entry = (name) => ({
    url: `https://github.com/${repo}/releases/download/${tag}/${name}`,
    signature: input.files.get(`${name}.sig`).toString("utf8"),
  });
  assert.deepEqual(manifest, {
    version: "0.22.0",
    notes: "Sürüm notları",
    platforms: {
      "darwin-aarch64": entry(mac),
      "darwin-x86_64": entry(mac),
      "darwin-aarch64-app": entry(mac),
      "darwin-x86_64-app": entry(mac),
      "windows-x86_64": entry(msi),
      "windows-x86_64-msi": entry(msi),
      "windows-x86_64-nsis": entry(nsis),
    },
  });
});

test("rejects a missing package, an incomplete upload, and a missing signature", async () => {
  const input = fixture();
  const assets = input.release.assets;
  input.release.assets = assets.filter(({ name }) => name !== dmg);
  await assert.rejects(buildUpdaterManifest(input), /macOS kurulum paketi/);
  input.release.assets = assets.map((asset) => asset.name === nsis ? { ...asset, state: "starter" } : asset);
  await assert.rejects(buildUpdaterManifest(input), /Windows NSIS/);
  input.release.assets = assets.filter(({ name }) => name !== `${msi}.sig`);
  await assert.rejects(buildUpdaterManifest(input), /imzası/);
});

test("rejects modified packages and signatures made with another key", async () => {
  const input = fixture();
  const original = input.files.get(nsis);
  input.files.set(nsis, Buffer.alloc(original.length, 13));
  await assert.rejects(buildUpdaterManifest(input), /paket imzası doğrulanamadı/);
  input.files.set(nsis, original);
  await assert.rejects(buildUpdaterManifest({ ...input, publicKey: signingKey().publicKey }), /paket imzası doğrulanamadı/);
});

test("authenticates the trusted comment in the Minisign envelope", async () => {
  const input = fixture();
  const name = `${mac}.sig`;
  const envelope = Buffer.from(input.files.get(name).toString("utf8"), "base64").toString("utf8");
  input.files.set(name, Buffer.from(Buffer.from(envelope.replace("timestamp:42", "timestamp:43")).toString("base64")));
  await assert.rejects(buildUpdaterManifest(input), /paket imzası doğrulanamadı/);
});

test("requires an unpublished stable draft for the exact tag and a single artifact per format", async () => {
  const input = fixture();
  await assert.rejects(buildUpdaterManifest({ ...input, tag: "v0.22.0-rc.1" }), /kararlı sürüm etiketi/);
  await assert.rejects(buildUpdaterManifest({ ...input, release: { ...input.release, isDraft: false } }), /yayın taslağı/);
  await assert.rejects(buildUpdaterManifest({ ...input, release: { ...input.release, tagName: "v0.21.0" } }), /yayın taslağı/);
  const duplicate = { name: `different_${nsis}`, size: 123, state: "uploaded" };
  input.release.assets.push(duplicate);
  await assert.rejects(buildUpdaterManifest(input), /Windows NSIS/);
});
