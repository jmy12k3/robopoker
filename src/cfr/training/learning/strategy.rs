use crate::cfr::training::learning::policy::Policy;
use crate::cfr::training::marker::action::Action;
use crate::cfr::training::marker::player::Player;
use crate::cfr::training::tree::node::Node;

/// A strategy (σ: player -> policy) is a function that assigns a policy to each h ∈ H, and therefore Ii ∈ Ii. Easily implemented as a HashMap<Info, Policy>.
pub(crate) trait Strategy {
    // required
    fn policy(&self, node: &Self::SNode) -> &Self::SPolicy;

    type SPlayer: Player;
    type SAction: Action;
    type SPolicy: Policy<PAction = Self::SAction>;
    type SNode: Node<NAction = Self::SAction> + Node<NPlayer = Self::SPlayer>;
}