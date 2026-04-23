import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ArrowRight, Plus, Power, Trash2 } from "lucide-react";
import { correctionCommands, type Correction } from "@/lib/corrections";
import { formatDateTime } from "@/utils/dateFormat";
import { Button } from "../../ui/Button";
import { SectionHeader } from "../SectionHeader";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-logo-primary hover:text-logo-primary/80"
        : "text-text/50 hover:text-logo-primary"
    }`}
    title={title}
  >
    {children}
  </button>
);

export const ReferencesSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [items, setItems] = useState<Correction[]>([]);
  const [loading, setLoading] = useState(true);
  const [newPhrase, setNewPhrase] = useState("");
  const [newExpansion, setNewExpansion] = useState("");
  const [adding, setAdding] = useState(false);

  const load = useCallback(async () => {
    const result = await correctionCommands.listCorrections(200, "reference");
    if (result.status === "ok") {
      setItems(result.data);
    } else {
      console.error("Failed to load references:", result.error);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleAdd = async () => {
    const phrase = newPhrase.trim();
    const expansion = newExpansion.trim();
    if (phrase.length === 0 || expansion.length === 0) return;
    if (phrase === expansion) {
      toast.error(t("settings.references.addMustDiffer"));
      return;
    }
    setAdding(true);
    const result = await correctionCommands.insertCorrection(
      phrase,
      expansion,
      "reference",
    );
    if (result.status === "ok") {
      setItems((prev) => [result.data, ...prev]);
      setNewPhrase("");
      setNewExpansion("");
    } else {
      toast.error(t("settings.references.addError"));
    }
    setAdding(false);
  };

  const handleToggle = async (item: Correction) => {
    setItems((prev) =>
      prev.map((i) => (i.id === item.id ? { ...i, enabled: !i.enabled } : i)),
    );
    const result = await correctionCommands.setCorrectionEnabled(
      item.id,
      !item.enabled,
    );
    if (result.status !== "ok") {
      setItems((prev) =>
        prev.map((i) =>
          i.id === item.id ? { ...i, enabled: item.enabled } : i,
        ),
      );
      toast.error(t("settings.references.toggleError"));
    }
  };

  const handleDelete = async (item: Correction) => {
    setItems((prev) => prev.filter((i) => i.id !== item.id));
    const result = await correctionCommands.deleteCorrection(item.id);
    if (result.status !== "ok") {
      load();
      toast.error(t("settings.references.deleteError"));
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto pb-12">
      <SectionHeader
        number="06"
        title={t("settings.references.title")}
        description={t("settings.references.description")}
      />
      <div className="space-y-2">
        <div className="px-4">
          {/* keeping the inline info card only, header is now in SectionHeader */}
          <div className="mt-2 rounded-md bg-logo-primary/5 border border-logo-primary/20 px-3 py-2 text-xs text-text/80">
            <span className="font-medium">
              {t("settings.references.exampleTitle")}:
            </span>{" "}
            {t("settings.references.exampleBody")}
          </div>
        </div>
        <div className="bg-background border border-mid-gray/20 rounded-lg px-4 py-3 flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-3">
          <input
            type="text"
            value={newPhrase}
            onChange={(e) => setNewPhrase(e.target.value)}
            placeholder={t("settings.references.addPhrasePlaceholder")}
            disabled={adding}
            className="flex-1 text-sm font-mono bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
          />
          <ArrowRight className="hidden sm:block w-4 h-4 shrink-0 text-text/40" />
          <input
            type="text"
            value={newExpansion}
            onChange={(e) => setNewExpansion(e.target.value)}
            placeholder={t("settings.references.addExpansionPlaceholder")}
            disabled={adding}
            className="flex-1 text-sm font-mono bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
          />
          <Button
            onClick={handleAdd}
            disabled={
              adding ||
              newPhrase.trim().length === 0 ||
              newExpansion.trim().length === 0
            }
            size="sm"
            className="flex items-center gap-1 shrink-0"
          >
            <Plus className="w-4 h-4" />
            <span>{t("settings.references.addButton")}</span>
          </Button>
        </div>
        <div className="bg-background border border-mid-gray/20 rounded-lg overflow-hidden">
          {loading ? (
            <div className="px-4 py-3 text-center text-text/60">
              {t("settings.references.loading")}
            </div>
          ) : items.length === 0 ? (
            <div className="px-4 py-6 text-center text-text/60 text-sm">
              {t("settings.references.empty")}
            </div>
          ) : (
            <ul className="divide-y divide-mid-gray/20">
              {items.map((item) => (
                <li key={item.id} className="px-4 py-3 flex items-center gap-3">
                  <div className="flex-1 min-w-0 flex items-center gap-2 text-sm">
                    <span
                      className={`font-mono px-2 py-0.5 rounded bg-mid-gray/10 truncate ${
                        item.enabled ? "" : "opacity-40 line-through"
                      }`}
                      title={item.original_text}
                    >
                      {item.original_text}
                    </span>
                    <ArrowRight className="w-4 h-4 shrink-0 text-text/40" />
                    <span
                      className={`font-mono px-2 py-0.5 rounded bg-logo-primary/10 truncate ${
                        item.enabled ? "" : "opacity-40 line-through"
                      }`}
                      title={item.corrected_text}
                    >
                      {item.corrected_text}
                    </span>
                  </div>
                  <span className="text-xs text-text/40 shrink-0 hidden sm:inline">
                    {formatDateTime(String(item.created_at), i18n.language)}
                  </span>
                  <div className="flex items-center">
                    <IconButton
                      onClick={() => handleToggle(item)}
                      active={item.enabled}
                      title={
                        item.enabled
                          ? t("settings.references.disable")
                          : t("settings.references.enable")
                      }
                    >
                      <Power className="w-4 h-4" />
                    </IconButton>
                    <IconButton
                      onClick={() => handleDelete(item)}
                      title={t("settings.references.delete")}
                    >
                      <Trash2 className="w-4 h-4" />
                    </IconButton>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
};
