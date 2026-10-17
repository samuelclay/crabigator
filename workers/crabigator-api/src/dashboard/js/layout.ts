// Session rows and expanded terminals share one reading column.
export const layoutJs = `
        function setLayout() {
            currentLayout = '1';
            const container = document.getElementById('sessions');
            container.dataset.layout = '1';
            container.style.columnCount = '';
            if (isFocusedMode()) applyFocusMode();
        }

        function updateFitLayout() { setLayout(); }
`;
