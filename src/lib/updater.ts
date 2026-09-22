import { check } from "@tauri-apps/plugin-updater";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";

/** Vérifie les releases GitHub et propose la mise à jour le cas échéant.
 *
 * `manual` (depuis le menu) : dit aussi « à jour » quand il n'y a rien, et
 * rapporte les erreurs réseau. Au démarrage (`manual` faux) : silence total
 * quand il n'y a rien à signaler — pas de notification pour un « non ».
 *
 * La signature de chaque installeur est vérifiée par le plugin avant
 * installation (minisign, clé publique dans tauri.conf.json) : une release
 * corrompue ou falsifiée est refusée.
 */
export async function checkForUpdates(manual = false): Promise<void> {
  let update;
  try {
    update = await check();
  } catch (e) {
    if (manual) {
      await message(`La vérification a échoué : ${e}`, {
        title: "Mise à jour",
        kind: "error",
      });
    }
    return;
  }

  if (!update) {
    if (manual) await message("Markdwn est à jour.", { title: "Mise à jour" });
    return;
  }

  const notes = update.body?.trim();
  const install = await ask(
    `Markdwn ${update.version} est disponible.${notes ? `\n\n${notes}` : ""}\n\nInstaller et relancer ?`,
    {
      title: "Mise à jour disponible",
      kind: "info",
      okLabel: "Installer",
      cancelLabel: "Plus tard",
    },
  );
  if (!install) return;

  try {
    await update.download();
    await update.install();
    // Sous Windows, install() lance l'installeur NSIS et quitte l'app :
    // relaunch ne s'exécute que sur les plateformes où elle survit.
    await relaunch();
  } catch (e) {
    await message(`L'installation a échoué : ${e}`, { title: "Mise à jour", kind: "error" });
  }
}
