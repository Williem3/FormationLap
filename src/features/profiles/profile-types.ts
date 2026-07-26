import type {
  DiscoverySnapshot,
  SupportingApplicationRecommendation,
} from "../../generated/bindings";

export type DiscoveryState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready"; snapshot: DiscoverySnapshot }
  | { kind: "error" };

export type RecommendationState =
  | { kind: "idle" }
  | { kind: "loading"; primarySimName: string }
  | {
      kind: "ready";
      primarySimName: string;
      recommendations: SupportingApplicationRecommendation[];
    }
  | { kind: "error"; primarySimName: string };

export type PrimarySimSource = "direct" | "steam";

export type ProfileApproval = {
  configurationReviewed: boolean;
  approvedPrivilegedApplicationIds: string[];
};
