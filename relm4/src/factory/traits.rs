//! Traits for for managing and updating factories.

use gtk::prelude::IsA;

use super::{DynamicIndex, FactoryComponentSender};
use crate::{OnDestroy, Sender};

use std::fmt::Debug;

/// A trait implemented for GTK4 widgets that allows a factory to create and remove widgets.
pub trait FactoryView: IsA<gtk::Widget> {
    /// The widget returned when inserting a widget.
    ///
    /// This doesn't matter on containers like [`gtk::Box`].
    /// However, widgets such as [`gtk::Stack`] return a [`gtk::StackPage`]
    /// which might be used to set additional parameters.
    ///
    /// Therefore, this "returned widget" is explicitly handled here.
    type ReturnedWidget: std::fmt::Debug + std::hash::Hash;

    /// Widget type that is attached to the container
    /// and also the root of the components.
    type Children: std::fmt::Debug + AsRef<Self::Children>;

    /// Position type used by this widget.
    ///
    /// For example [`GridPosition`](super::positions::GridPosition) for [`gtk::Grid`] or `()` for [`gtk::Box`]
    type Position;

    /// Removes a widget.
    fn factory_remove(&self, widget: &Self::ReturnedWidget);
    /// Adds a new widget to self at the end.
    fn factory_append(
        &self,
        widget: impl AsRef<Self::Children>,
        position: &Self::Position,
    ) -> Self::ReturnedWidget;

    /// Add an widget to the front.
    fn factory_prepend(
        &self,
        widget: impl AsRef<Self::Children>,
        position: &Self::Position,
    ) -> Self::ReturnedWidget;

    /// Insert a widget after another widget.
    fn factory_insert_after(
        &self,
        widget: impl AsRef<Self::Children>,
        position: &Self::Position,
        other: &Self::ReturnedWidget,
    ) -> Self::ReturnedWidget;

    /// Converts a returned widget to the children type.
    ///
    fn returned_widget_to_child(root_child: &Self::ReturnedWidget) -> Self::Children;

    /// Move an item after another item.
    fn factory_move_after(&self, widget: &Self::ReturnedWidget, other: &Self::ReturnedWidget);

    /// Move an item to the start.
    fn factory_move_start(&self, widget: &Self::ReturnedWidget);

    /// Update the position inside positioned containers like [`gtk::Grid`].
    fn factory_update_position(&self, _widget: &Self::ReturnedWidget, _position: &Self::Position) {}
}

/// Returns the position of an element inside a
/// container like [`gtk::Grid`] where the position isn't
/// clearly defined by the index.
pub trait Position<Pos> {
    /// Returns the position.
    ///
    /// This function can be called very often
    /// if widgets are moved a lot, so it should
    /// be cheap to call.
    fn position(index: usize) -> Pos;
}

impl<C> Position<()> for C {
    fn position(_index: usize) {}
}

/// A component that's stored inside a factory.
/// Similar to [`Component`](crate::Component) but adjusted to fit the life cycle
/// of factories.
pub trait FactoryComponent:
    Position<<Self::ParentWidget as FactoryView>::Position> + Sized + Debug + 'static
{
    /// Container widget to which all widgets of the factory will be added.
    type ParentWidget: FactoryView + 'static;

    /// Input messages sent to the parent component.
    type ParentInput: Debug + 'static;

    /// Messages which are received from commands executing in the background.
    type CommandOutput: Debug + Send + 'static;

    /// The message type that the factory component accepts as inputs.
    type Input: Debug + 'static;

    /// The message type that the factory component provides as outputs.
    type Output: Debug + 'static;

    /// The parameter used to initialize the factory component.
    type Init;

    /// The widget that was constructed by the factory component.
    type Root: AsRef<<Self::ParentWidget as FactoryView>::Children> + Debug + OnDestroy + Clone;

    /// The type that's used for storing widgets created for this factory component.
    type Widgets: 'static;

    /// Initializes the model.
    fn init_model(
        init: Self::Init,
        index: &DynamicIndex,
        sender: FactoryComponentSender<Self>,
    ) -> Self;

    /// Initializes the root widget
    fn init_root(&self) -> Self::Root;

    /// Initializes the widgets.
    fn init_widgets(
        &mut self,
        index: &DynamicIndex,
        root: &Self::Root,
        returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactoryComponentSender<Self>,
    ) -> Self::Widgets;

    /// Optionally convert an output message from this component to an input message for the
    /// parent component.
    ///
    /// If [`None`] is returned, nothing is forwarded.
    fn output_to_parent_input(_output: Self::Output) -> Option<Self::ParentInput> {
        None
    }

    /// Processes inputs received by the component.
    #[allow(unused)]
    fn update(&mut self, message: Self::Input, sender: FactoryComponentSender<Self>) {}

    /// Defines how the component should respond to command updates.
    #[allow(unused)]
    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactoryComponentSender<Self>) {}

    /// Handles updates from a command.
    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: FactoryComponentSender<Self>,
    ) {
        self.update_cmd(message, sender.clone());
        self.update_view(widgets, sender);
    }

    /// Updates the view after the model has been updated.
    #[allow(unused)]
    fn update_view(&self, widgets: &mut Self::Widgets, sender: FactoryComponentSender<Self>) {}

    /// Updates the model and view. Optionally returns a command to run.
    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: FactoryComponentSender<Self>,
    ) {
        self.update(message, sender.clone());
        self.update_view(widgets, sender);
    }

    /// Last method called before a component is shut down.
    #[allow(unused)]
    fn shutdown(&mut self, widgets: &mut Self::Widgets, output: Sender<Self::Output>) {}

    /// An identifier for the component used for debug logging.
    ///
    /// The default implementation of this method uses the address of the component, but
    /// implementations are free to provide more meaningful identifiers.
    fn id(&self) -> String {
        format!("{:p}", &self)
    }
}
