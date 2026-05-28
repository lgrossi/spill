export type RetroPhase = "writing" | "discussion" | "voting" | "action_discussion" | "completed";

export type RetroSummary = {
  id: string;
  title: string;
  phase: RetroPhase;
  vote_limit: number;
  action_discussion_limit: number;
  created_at: string;
  last_activity_at: string;
  last_opened_at: string | null;
  participant_count: number;
  ready_count: number;
  column_count: number;
  unresolved_action_count: number;
  recurring_tags: string[];
  open_actions: RetroActionSummary[];
};

export type RetroActionSummary = {
  id: string;
  title: string;
  status: "proposed" | "confirmed";
};

export type RetroOverview = {
  active: RetroSummary[];
  completed: RetroSummary[];
};

export type RetroColumn = {
  id: string;
  retro_id: string;
  column_key: string;
  title: string;
  position: number;
  order_direction: "chronological" | "reverse_chronological";
  accent_color?: string | null;
  cards: RetroCard[];
};

export type RetroCard = {
  id: string;
  retro_id: string;
  column_id: string;
  author_participant_id: string;
  body_text: string | null;
  gif_url: string | null;
  gif_alt_text: string | null;
  state: "draft" | "revealed";
  position: number;
  hidden: boolean;
  vote_count: number;
  current_user_vote_count: number;
  cluster_id: string | null;
  parent_card_id: string | null;
  cluster_details: string | null;
  cluster_title: string | null;
  cluster_category: string | null;
  cluster_members: {
    id: string;
    author_participant_id: string;
    body_text: string | null;
    gif_url: string | null;
    gif_alt_text: string | null;
    hidden: boolean;
  }[];
};

export type RetroParticipant = {
  id: string;
  retro_id: string;
  external_subject: string | null;
  display_name: string;
  role: "host" | "member";
};

export type GifResult = {
  id: string;
  url: string;
  preview_url: string;
  alt_text: string;
  media_type: "image" | "video";
  kind: "all" | "gif" | "sticker" | "clip";
};

export type GifSearchResponse = {
  results: GifResult[];
  degraded: boolean;
};

export type RetroBoard = {
  retro: {
    id: string;
    title: string;
    phase: RetroPhase;
    vote_limit: number;
    action_discussion_limit: number;
  };
  participants: RetroParticipant[];
  columns: RetroColumn[];
  ready: {
    participant_count: number;
    ready_count: number;
    current_user_ready: boolean;
  };
  voting: {
    vote_limit: number;
    votes_used: number;
    votes_remaining: number;
  };
  clusters: {
    id: string;
    retro_id: string;
    title: string | null;
    category: string | null;
    tags: string[];
  }[];
  actions: RetroActionItem[];
  deck: IngestedItem[];
  ai_artifacts: AiArtifact[];
  meeting_notes: MeetingNote[];
  deliveries: Delivery[];
};

export type RetroActionItem = {
  id: string;
  retro_id: string;
  source_card_id: string | null;
  source_cluster_id: string | null;
  title: string;
  details: string | null;
  status: "proposed" | "confirmed" | "rejected" | "done";
  position: number;
  tags: string[];
};

export type IngestedItem = {
  id: string;
  retro_id: string | null;
  source: "pi" | "claude_code" | "upload" | "other";
  placement: "user_deck" | "retro_draft";
  target_column_id: string | null;
  suggested_text: string | null;
  gif_url: string | null;
  status: "pending" | "accepted" | "dismissed";
};

export type AiArtifact = {
  id: string;
  retro_id: string;
  kind: "gif_suggestions" | "clustering" | "action_suggestions" | "summary" | "mood" | "tagging";
  status: "pending" | "running" | "succeeded" | "failed";
  input: unknown;
  output: unknown | null;
  error_message: string | null;
  retry_count: number;
};

export type MeetingNote = {
  id: string;
  retro_id: string;
  title: string;
  body_text: string;
};

export type Delivery = {
  id: string;
  retro_id: string;
  kind: "summary_export" | "external_action_link";
  status: "pending" | "succeeded" | "failed";
  output: unknown | null;
  error_message: string | null;
  retry_count: number;
};

export type InviteeRequest = {
  email: string;
  role?: "host" | "member";
};

export type CreateRetroPayload =
  | {
      title: string;
      template: "standard";
      vote_limit: number;
      action_discussion_limit: number;
      invitees?: InviteeRequest[];
    }
  | {
      title: string;
      template: "custom";
      columns: string[];
      column_colors?: string[];
      vote_limit: number;
      action_discussion_limit: number;
      invitees?: InviteeRequest[];
    };

export type Grant = {
  id: string;
  retro_id: string;
  principal_email: string;
  role: "host" | "member";
};

/** Tag values injected automatically by the system; filter these before showing user-facing tag counts. */
export const SYSTEM_RECURRING_TAGS = new Set(["topvoted", "auto-clustered"]);
