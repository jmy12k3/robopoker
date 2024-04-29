#![allow(dead_code)]

/// Regret Minimization in Games with Incomplete Information. Advances in Neural Information Processing Systems, 20.
/// Zinkevich, M., Bowling, M., Burch, N., Cao, Y., Johanson, M., Tamblyn, I., & Rocco, M. (2007).

// Marker types
type Utility = f32;
type Probability = f32;

// A finite set N of players, including chance
trait Player {}

// A finite set of possible actions
trait Action {
    type Player;

    fn player(&self) -> &Self::Player;
    fn belongs(&self, _: &Self::Player) -> bool;
}

// Omnipotent, complete state of current game
trait Node {
    type Action: Action<Player = Self::Player>;
    type Player: Player;

    // fn parent(&self) -> Option<&Self>;
    fn value(&self, _: &Self::Player) -> Utility;
    fn player(&self) -> &Self::Player;
    fn history(&self) -> &Vec<&Self::Action>;
    fn available(&self) -> &Vec<&Self::Action>;
    fn children(&self) -> &Vec<&Self>;

    fn descendants(&self) -> Vec<&Self> {
        match self.children().len() {
            0 => vec![&self],
            _ => self
                .children()
                .iter()
                .map(|child| child.descendants())
                .flatten()
                .collect(),
        }
    }
}

// All known information at a given node, up to any abstractions. Think of it as a distribution over the unknown game state.
trait Info {
    type Node: Node<Player = Self::Player, Action = Self::Action>;
    type Action: Action<Player = Self::Player>;
    type Player: Player;

    fn possibles(&self) -> &Vec<&Self::Node>;

    fn endpoints(&self) -> Vec<&Self::Node> {
        self.possibles()
            .iter()
            .map(|node| node.descendants())
            .flatten()
            .collect()
    }
    fn player(&self) -> &Self::Player {
        self.possibles().iter().next().unwrap().player()
    }
    fn available(&self) -> &Vec<&Self::Action> {
        self.possibles().iter().next().unwrap().available()
    }
}

// A policy is a distribution over A(Ii)
trait Policy {
    type Action: Action<Player = Self::Player>;
    type Player: Player;

    fn weight(&self, _: &Self::Action) -> Probability;
}

// A strategy of player i σi in an extensive game is a function that assigns a policy to each Ii ∈ Ii
trait Strategy {
    type Policy: Policy<Player = Self::Player, Action = Self::Action>;
    type Info: Info<Player = Self::Player, Action = Self::Action, Node = Self::Node>;
    type Node: Node<Player = Self::Player, Action = Self::Action>;
    type Action: Action<Player = Self::Player>;
    type Player: Player;

    fn policy(&self, _: &Self::Node) -> &Self::Policy;
}

// A profile σ consists of a strategy for each player, σ1,σ2,..., equivalently a matrix indexed by (player, action) or (i,a) ∈ N × A
trait Profile {
    type Strategy: Strategy<
        Player = Self::Player,
        Action = Self::Action,
        Node = Self::Node,
        Info = Self::Info,
    >;
    type Info: Info<Player = Self::Player, Action = Self::Action, Node = Self::Node>;
    type Node: Node<Player = Self::Player, Action = Self::Action>;
    type Action: Action<Player = Self::Player>;
    type Player: Player;

    /// Return a Profile where info.player's strategy is to play P(action)= 100%
    fn always(&self, action: &Self::Action) -> Self;
    /// Return a Profile where info.player's strategy is given
    fn replace(&self, strategy: Self::Strategy) -> Self;
    /// Return the strategy for player i
    fn strategy(&self, _: &Self::Player) -> &Self::Strategy;

    /// EV for info.player iff players play according to &self
    fn expected_value(&self, info: &Self::Info /* player */) -> Utility {
        info.endpoints()
            .iter()
            .map(|end| end.value(info.player()) * self.reach(end))
            .sum()
    }
    /// EV for info.player iff players play according to &self BUT info.player plays according to P(action)= 100%. we can interpret this as a dot product between optimal strategy and current strategy
    fn cfactual_value(&self, info: &Self::Info /* player */) -> Utility {
        info.possibles()
            .iter()
            .map(|root| {
                root.descendants()
                    .iter()
                    .map(|leaf| {
                        leaf.value(info.player()/*  */) // V ( LEAF )
                            * self.exterior_reach(root) // P ( ROOT | player tried to reach INFO )
                            * self.relative_reach(root, leaf) // P ( ROOT -> LEAF )
                    })
                    .sum::<Utility>()
            })
            .sum::<Utility>()
            / info
                .possibles()
                .iter()
                .map(|root| self.reach(root))
                .sum::<Utility>() //? DIV BY ZERO
    }
    // reach probabilities
    fn reach(&self, node: &Self::Node) -> Probability {
        node.history()
            .iter()
            .map(|action| self.strategy(action.player()).policy(node).weight(action))
            .product()
    }
    fn exterior_reach(&self, node: &Self::Node) -> Probability {
        node.history()
            .iter()
            .filter(|action| !!!action.belongs(node.player()))
            .map(|action| self.strategy(action.player()).policy(node).weight(action))
            .product()
    }
    fn relative_reach(&self, root: &Self::Node, leaf: &Self::Node) -> Probability {
        self.reach(leaf) / self.reach(root) //? DIV BY ZERO
    }
}

// Training happens over discrete time steps, so we'll index steps into it's own data structure.xz
trait Step {
    type Profile: Profile<
        Player = Self::Player,
        Action = Self::Action,
        Node = Self::Node,
        Info = Self::Info,
        Strategy = Self::Strategy,
    >;
    type Strategy: Strategy<
        Player = Self::Player,
        Action = Self::Action,
        Node = Self::Node,
        Info = Self::Info,
    >;
    type Info: Info<Player = Self::Player, Action = Self::Action, Node = Self::Node>;
    type Node: Node<Player = Self::Player, Action = Self::Action>;
    type Action: Action<Player = Self::Player>;
    type Player: Player;

    fn new(info: &Self::Info, profile: Self::Profile) -> Self;
    fn info(&self) -> &Self::Info;
    fn profile(&self) -> &Self::Profile; //? owned by step or solver? mutable or immutable?

    /// aka instantaneous regret. we call step-regret loss and solver-regret regret, the latter is cumulative
    fn loss(&self, action: &Self::Action) -> Utility {
        // let info = self.info();
        // let player = info.player();
        self.profile()
            .always(action)
            .cfactual_value(self.info() /* , player */)
            - self.profile().cfactual_value(self.info() /* , player */)
    }
}

// A full solver has a sequence of steps, and a final profile
trait Solver {
    type Step: Step<
        Player = Self::Player,
        Action = Self::Action,
        Node = Self::Node,
        Info = Self::Info,
        Strategy = Self::Strategy,
        Profile = Self::Profile,
    >;
    type Profile: Profile<
        Player = Self::Player,
        Action = Self::Action,
        Node = Self::Node,
        Info = Self::Info,
        Strategy = Self::Strategy,
    >;
    type Strategy: Strategy<
        Player = Self::Player,
        Action = Self::Action,
        Node = Self::Node,
        Info = Self::Info,
    >;
    type Info: Info<Player = Self::Player, Action = Self::Action, Node = Self::Node>;
    type Node: Node<Player = Self::Player, Action = Self::Action>;
    type Action: Action<Player = Self::Player>;
    type Player: Player;

    fn info(&self) -> &Self::Info;
    fn steps(&self) -> &mut Vec<Self::Step>;
    fn profile(&self) -> &mut Self::Profile; //? owned by step or solver? mutable or immutable?
    fn next_strategy(&self) -> Self::Strategy;

    /// aka average cumulative regret. we call step-regret loss and solver-regret regret, the latter is cumulative
    fn regret(&self, action: &Self::Action) -> Utility {
        self.steps()
            .iter()
            .map(|step| step.loss(action))
            .sum::<Utility>()
            / self.num_steps() as Utility //? DIV BY ZERO
    }
    /// Loops over simple n_iter < max_iter convergence criteria and returns ~Nash Best Response
    fn solve(&mut self) -> &Self::Strategy {
        while let Some(step) = self.next() {
            self.steps().push(step);
        }
        self.steps()
            .last()
            .unwrap()
            .profile()
            .strategy(self.info().player())
    }
    /// Generate the next Step of the solution as a pure function of current state
    fn next(&self) -> Option<Self::Step> {
        if self.num_steps() < self.max_steps() {
            Some(Self::Step::new(self.info(), self.next_profile()))
        } else {
            None
        }
    }
    /// Apply regret minimization via regret matching
    fn next_profile(&self) -> Self::Profile {
        self.steps()
            .last()
            .unwrap()
            .profile()
            .replace(self.next_strategy())
    }
    /// Convergence progress
    fn num_steps(&self) -> usize {
        self.steps().len()
    }
    fn max_steps(&self) -> usize {
        10_000
    }
}
