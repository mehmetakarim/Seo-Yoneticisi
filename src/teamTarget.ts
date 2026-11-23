import { computed, ref, watch } from "vue";
import { teamAccessIssue, teamClient, teamConnectionRetry, teamReachable, teamServiceStatus, teamSession, teamUser, teamWorkspace } from "./teamConnection";

export function useTeamTarget() {
  const workspaceName = ref("");
  const role = ref("");
  const checking = ref(false);
  const lookupError = ref("");
  const attempt = ref(0);
  const connected = computed(() => !!(teamClient.value && teamSession.value && teamWorkspace.value));
  const canWrite = computed(() => !!workspaceName.value && ["owner", "editor"].includes(role.value)
    && teamReachable.value === true && !teamAccessIssue.value);
  const statusMessage = computed(() => {
    if (teamServiceStatus.value !== null) return `Supabase şu anda HTTP ${teamServiceStatus.value} döndürüyor. Bağlantı geri geldiğinde çalışma alanı tekrar doğrulanacak.`;
    if (teamAccessIssue.value) return "Bu çalışma alanına erişim doğrulanamadı.";
    if (teamReachable.value === false && teamUser.value) return "Supabase bağlantısı bekleniyor. Bağlantı geri geldiğinde çalışma alanı tekrar doğrulanacak.";
    return lookupError.value;
  });

  watch([teamClient, teamWorkspace, () => teamSession.value?.user.id, teamReachable, teamAccessIssue, attempt],
    async ([client, workspace, user, reachable, accessIssue], _, cleanup) => {
      let current = true;
      cleanup(() => { current = false; });
      workspaceName.value = "";
      role.value = "";
      lookupError.value = "";
      checking.value = false;
      if (!client || !workspace || !user || !reachable || accessIssue) return;
      checking.value = true;
      try {
        const [space, member] = await Promise.all([
          client.from("workspaces").select("name").eq("id", workspace).maybeSingle().retry(false),
          client.from("workspace_members").select("role").eq("workspace_id", workspace).eq("user_id", user).maybeSingle().retry(false),
        ]);
        if (!current) return;
        if (space.error || member.error || !space.data || !member.data) throw new Error();
        workspaceName.value = space.data.name;
        role.value = member.data.role;
      } catch {
        if (current) lookupError.value = "Hedef çalışma alanı doğrulanamadı. Yeniden deneyebilirsiniz.";
      } finally {
        if (current) checking.value = false;
      }
    }, { immediate: true });
  return { workspaceName, connected, canWrite, checking, statusMessage, retry: () => {
    if (teamReachable.value !== true || teamAccessIssue.value) teamConnectionRetry.value++;
    else attempt.value++;
  } };
}
