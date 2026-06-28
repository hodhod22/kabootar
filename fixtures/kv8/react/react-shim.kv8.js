let React = {
  createElement: (tag, props, childText) => {
    let el = document.createElement(tag);
    if (props) {
      if (props.id) { el.id = props.id; }
      if (props.className) { el.setAttribute('class', props.className); }
      if (props.onClick) { el.addEventListener('click', props.onClick); }
    }
    if (childText) { el.textContent = childText; }
    return el;
  }
};

let ReactDOM = {
  createRoot: (container) => {
    return {
      render: (vnode) => {
        while (container.firstChild) {
          container.removeChild(container.firstChild);
        }
        container.appendChild(vnode);
      }
    };
  }
};
