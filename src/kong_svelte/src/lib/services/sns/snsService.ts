import { Actor, HttpAgent } from '@dfinity/agent';
import { idlFactory } from '$lib/idls/snsGovernance.idl.js';
import { createAnonymousActorHelper } from '$lib/utils/actorUtils';

export interface SNSProposal {
  id: bigint;
  title: string;
  summary: string;
  url: string;
  status: 'open' | 'accepted' | 'rejected' | 'executing' | 'executed' | 'failed';
  proposer?: string;
  created: bigint;
  deadline: bigint;
  reward_event_round: bigint;
  tally: {
    yes: bigint | undefined;
    no: bigint | undefined;
    total: bigint | undefined;
    timestamp_seconds: bigint | undefined;
  };
  decided_timestamp_seconds: bigint;
  executed_timestamp_seconds: bigint;
  failed_timestamp_seconds: bigint;
  is_eligible_for_rewards: boolean;
}

export interface ProposalResponse {
  proposals: SNSProposal[];
  hasMore: boolean;
}

export const GOVERNANCE_CANISTER_IDS: { [key: string]: string } = {
  "o7oak-iyaaa-aaaaq-aadzq-cai": "oypg6-faaaa-aaaaq-aadza-cai"
}

export class SNSService {
  private agent: HttpAgent;

  constructor() {
    this.agent = HttpAgent.createSync({
      host: 'https://ic0.app'
    });
  }

  async getProposals(
    governanceCanisterId: string, 
    limit: number = 10,
    beforeProposal?: bigint
  ): Promise<ProposalResponse> {
    try {
      const governanceActor = createAnonymousActorHelper(governanceCanisterId, idlFactory);

      const result = await governanceActor.list_proposals({
        limit: BigInt(limit),
        before_proposal: beforeProposal ? [beforeProposal] : [],
        exclude_type: [],
        include_reward_status: [] as number[],
        include_status: [] as number[]
      });

      function determineStatus(proposal: any): SNSProposal['status'] {
        if (proposal.failed_timestamp_seconds > 0n) return 'failed';
        if (proposal.executed_timestamp_seconds > 0n) return 'executed';
        if (proposal.decided_timestamp_seconds > 0n) return 'accepted';
        return 'open';
      }

      const mappedProposals = result.proposals.map(proposal => ({
        id: proposal.id[0]?.id || BigInt(0),
        title: proposal.proposal[0]?.title || "",
        summary: proposal.proposal[0]?.summary || "",
        url: proposal.proposal[0]?.url || "",
        status: determineStatus(proposal),
        proposer: proposal.proposer[0]?.id.toString(),
        created: proposal.proposal_creation_timestamp_seconds,
        deadline: proposal.wait_for_quiet_deadline_increase_seconds,
        reward_event_round: proposal.reward_event_round,
        tally: {
          yes: proposal.latest_tally[0]?.yes,
          no: proposal.latest_tally[0]?.no,
          total: proposal.latest_tally[0]?.total,
          timestamp_seconds: proposal.latest_tally[0]?.timestamp_seconds
        },
        decided_timestamp_seconds: proposal.decided_timestamp_seconds,
        executed_timestamp_seconds: proposal.executed_timestamp_seconds,
        failed_timestamp_seconds: proposal.failed_timestamp_seconds,
        is_eligible_for_rewards: proposal.is_eligible_for_rewards
      }));

      return {
        proposals: mappedProposals,
        hasMore: result.proposals.length >= limit
      };
    } catch (error) {
      console.error('Error fetching SNS proposals:', error);
      return { proposals: [], hasMore: false };
    }
  }
}

export const snsService = new SNSService(); 