document.addEventListener("alpine:init", () => {
  Alpine.data("expenseApp", () => ({
    filter: "all",
    query: "",
    selected: null,
    visible: 0,
    toast: { visible: false, message: "" },
    toastTimer: null,
    dragStartY: 0,
    dragOffset: 0,

    flowOpen: false,
    step: 0,
    total: 6,
    returnToReview: false,
    submitting: false,
    today: "",
    dates: [],
    calYear: 0,
    calMonth: 0,
    category: "",
    categoryLabel: "",
    categoryIcon: "i-briefcase",
    categoryTone: "secondary",
    date: "",
    otherDate: false,
    currencyCode: "GBP",
    currencySymbol: "£",
    currencyName: "British Pound",
    currencyOpen: false,
    currencyOther: false,
    otherCode: "",
    description: "",
    amount: "",
    receiptName: "",
    receiptSize: "",
    receiptUrl: "",
    receiptIsImage: false,

    init() {
      this.refreshDates();
      this.$watch("selected", () => this.syncLock());
      this.$watch("flowOpen", () => this.syncLock());

      document.body.addEventListener("htmx:afterSettle", () => {
        this.$nextTick(() => {
          this.reconcileFilter();
          this.apply();
          this.localizeTimes();
        });
      });
      document.body.addEventListener("htmx:beforeRequest", (event) => {
        if (!this.isForm(event)) return;
        if (!this.canSubmit) {
          event.preventDefault();
          return;
        }
        this.submitting = true;
      });
      document.body.addEventListener("htmx:afterRequest", (event) => {
        if (this.isForm(event)) {
          this.submitting = false;
          if (event.detail.successful) this.finish();
        }
        if (!this.isAction(event) || !window.haptics) return;
        if (event.detail.successful) window.haptics.success();
        else window.haptics.error();
      });
      document.body.addEventListener("htmx:responseError", (event) => {
        if (!this.isAction(event)) return;
        const xhr = event.detail && event.detail.xhr;
        if (xhr && xhr.getResponseHeader("HX-Trigger")) return;
        this.notify("Something went wrong. Please try again.");
      });
      document.body.addEventListener("htmx:sendError", (event) => {
        if (this.isAction(event)) this.notify("You appear to be offline.");
      });
      document.body.addEventListener("toast", (event) => {
        this.notify(event.detail && event.detail.message);
      });

      this.$nextTick(() => {
        this.localizeTimes();
        this.openFromLink();
      });
    },

    get locked() {
      return this.flowOpen || Boolean(this.selected);
    },

    get progress() {
      return ((this.step + 1) / this.total) * 100;
    },

    get dateValid() {
      return /^\d{4}-\d{2}-\d{2}$/.test(this.date) && this.date <= this.today;
    },

    get descriptionValid() {
      return this.description.trim().length > 0;
    },

    get amountValid() {
      return (
        /^\d{1,7}(\.\d{1,2})?$/.test(this.amount) && Number(this.amount) > 0
      );
    },

    get currencyValid() {
      return /^[A-Z]{3}$/.test(this.currencyCode);
    },

    get canContinue() {
      switch (this.step) {
        case 0:
          return Boolean(this.category);
        case 1:
          return this.dateValid;
        case 2:
          return this.descriptionValid;
        case 3:
          return this.amountValid && this.currencyValid;
        case 4:
          return Boolean(this.receiptName);
        default:
          return false;
      }
    },

    get showContinue() {
      return this.step >= 1 && this.step <= 4;
    },

    get canSubmit() {
      return (
        this.step === 5 &&
        !this.submitting &&
        Boolean(this.category) &&
        this.dateValid &&
        this.descriptionValid &&
        this.amountValid &&
        this.currencyValid &&
        Boolean(this.receiptName)
      );
    },

    get currencyGlyph() {
      return this.currencySymbol || "¤";
    },

    get currencyLabel() {
      if (this.currencyOther) {
        return this.currencyCode
          ? `${this.currencyCode} · Other currency`
          : "Other currency";
      }
      return `${this.currencyCode} · ${this.currencyName}`;
    },

    get amountText() {
      const number = Number(this.amount || 0).toLocaleString("en-GB", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      });
      const symbol = this.currencySymbol || this.currencyCode;
      return /^[A-Za-z]+$/.test(symbol)
        ? `${symbol} ${number}`
        : `${symbol}${number}`;
    },

    get dateText() {
      return this.formatDate(this.date);
    },

    get monthIndex() {
      return this.calYear * 12 + this.calMonth;
    },

    get todayIndex() {
      const now = this.today ? this.parseIso(this.today) : new Date();
      return now.getFullYear() * 12 + now.getMonth();
    },

    get canNextMonth() {
      return this.monthIndex < this.todayIndex;
    },

    get canPrevMonth() {
      return this.todayIndex - this.monthIndex < 36;
    },

    get calendarTitle() {
      return new Date(this.calYear, this.calMonth, 1).toLocaleDateString(
        "en-GB",
        {
          month: "long",
          year: "numeric",
        },
      );
    },

    get calendarCells() {
      const first = new Date(this.calYear, this.calMonth, 1);
      const offset = (first.getDay() + 6) % 7;
      const count = new Date(this.calYear, this.calMonth + 1, 0).getDate();
      const cells = [];
      for (let i = 0; i < offset; i += 1) {
        cells.push({ key: `blank-${i}`, blank: true });
      }
      for (let day = 1; day <= count; day += 1) {
        const value = this.iso(new Date(this.calYear, this.calMonth, day));
        cells.push({
          key: value,
          blank: false,
          day,
          value,
          future: value > this.today,
          today: value === this.today,
        });
      }
      return cells;
    },

    isAction(event) {
      const config = event.detail && event.detail.requestConfig;
      return Boolean(config && config.verb === "post");
    },

    isForm(event) {
      return Boolean(
        event.detail &&
        event.detail.elt &&
        event.detail.elt.id === "expense-form",
      );
    },

    apply() {
      const query = this.query.trim().toLowerCase();
      let count = 0;
      this.$root.querySelectorAll("[data-expense]").forEach((row) => {
        const matchesFilter =
          this.filter === "all" || this.filter === row.dataset.category;
        const matchesQuery = !query || row.dataset.search.includes(query);
        const show = matchesFilter && matchesQuery;
        row.hidden = !show;
        if (show) count += 1;
      });
      this.$root.querySelectorAll("[data-group]").forEach((group) => {
        group.hidden = !group.querySelector("[data-expense]:not([hidden])");
      });
      this.visible = count;
    },

    reconcileFilter() {
      if (this.filter === "all") return;
      const chip = this.$root.querySelector(
        `[data-chip="${CSS.escape(this.filter)}"]`,
      );
      if (!chip) this.filter = "all";
    },

    localizeTimes() {
      this.$root.querySelectorAll("time[data-local-time]").forEach((el) => {
        const date = new Date(el.getAttribute("datetime"));
        if (Number.isNaN(date.getTime())) return;
        const day = date.toLocaleDateString("en-GB", {
          day: "numeric",
          month: "short",
          year: "numeric",
        });
        const time = date.toLocaleTimeString("en-GB", {
          hour: "2-digit",
          minute: "2-digit",
          hour12: false,
        });
        el.textContent = `${day}, ${time}`;
      });
    },

    open(id) {
      this.selected = id;
    },

    close() {
      this.selected = null;
    },

    syncLock() {
      document.body.classList.toggle("is-locked", this.locked);
      if (!window.tgApp) return;
      if (this.flowOpen) window.tgApp.setBack(() => this.back());
      else if (this.selected) window.tgApp.setBack(() => this.close());
      else window.tgApp.setBack(null);
    },

    openFromLink() {
      const params = new URLSearchParams(window.location.search);
      const id =
        params.get("expense") || (window.tgApp && window.tgApp.startParam());
      if (!id) return;
      const exists = this.$root.querySelector(
        `[data-sheet="${CSS.escape(id)}"]`,
      );
      if (exists) this.selected = id;
    },

    notify(message) {
      if (!message) return;
      this.toast.message = message;
      this.toast.visible = true;
      clearTimeout(this.toastTimer);
      this.toastTimer = setTimeout(() => {
        this.toast.visible = false;
      }, 2800);
    },

    iso(date) {
      const pad = (n) => String(n).padStart(2, "0");
      return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
    },

    parseIso(value) {
      const [y, m, d] = value.split("-").map(Number);
      return new Date(y, m - 1, d);
    },

    refreshDates() {
      const now = new Date();
      const list = [];
      for (let i = 0; i < 5; i += 1) {
        const day = new Date(
          now.getFullYear(),
          now.getMonth(),
          now.getDate() - i,
        );
        let caption = day.toLocaleDateString("en-GB", { weekday: "long" });
        if (i === 0) caption = "Today";
        if (i === 1) caption = "Yesterday";
        list.push({
          value: this.iso(day),
          label: day.toLocaleDateString("en-GB", {
            day: "numeric",
            month: "short",
          }),
          caption,
        });
      }
      this.dates = list;
      this.today = list[0].value;
    },

    formatDate(value) {
      if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return "";
      return this.parseIso(value).toLocaleDateString("en-GB", {
        weekday: "short",
        day: "numeric",
        month: "short",
        year: "numeric",
      });
    },

    formatSize(bytes) {
      if (bytes < 1024 * 1024)
        return `${Math.max(1, Math.round(bytes / 1024))} KB`;
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    },

    showMonthOf(value) {
      const base = /^\d{4}-\d{2}-\d{2}$/.test(value)
        ? this.parseIso(value)
        : new Date();
      this.calYear = base.getFullYear();
      this.calMonth = base.getMonth();
    },

    syncDateView() {
      const inList = this.dates.some((d) => d.value === this.date);
      this.otherDate = Boolean(this.date) && !inList;
      this.showMonthOf(this.date || this.today);
    },

    prevMonth() {
      if (!this.canPrevMonth) return;
      if (this.calMonth === 0) {
        this.calMonth = 11;
        this.calYear -= 1;
      } else {
        this.calMonth -= 1;
      }
    },

    nextMonth() {
      if (!this.canNextMonth) return;
      if (this.calMonth === 11) {
        this.calMonth = 0;
        this.calYear += 1;
      } else {
        this.calMonth += 1;
      }
    },

    startFlow() {
      this.resetFlow();
      this.refreshDates();
      this.date = this.today;
      this.flowOpen = true;
    },

    closeFlow() {
      this.flowOpen = false;
    },

    finish() {
      this.flowOpen = false;
      setTimeout(() => this.resetFlow(), 500);
    },

    resetFlow() {
      this.step = 0;
      this.returnToReview = false;
      this.submitting = false;
      this.category = "";
      this.categoryLabel = "";
      this.categoryIcon = "i-briefcase";
      this.categoryTone = "secondary";
      this.date = "";
      this.otherDate = false;
      this.currencyCode = "GBP";
      this.currencySymbol = "£";
      this.currencyName = "British Pound";
      this.currencyOpen = false;
      this.currencyOther = false;
      this.otherCode = "";
      this.description = "";
      this.amount = "";
      this.clearReceipt();
    },

    go(step) {
      this.step = step;
      if (step === 1) this.syncDateView();
      this.currencyOpen = false;
      this.$nextTick(() => {
        if (this.$refs.flowBody) this.$refs.flowBody.scrollTop = 0;
        const target =
          step === 2
            ? this.$refs.description
            : step === 3
              ? this.$refs.amount
              : null;
        if (target) target.focus();
      });
    },

    goTo(step) {
      this.returnToReview = true;
      this.go(step);
    },

    advance() {
      if (this.returnToReview) {
        this.returnToReview = false;
        this.go(this.total - 1);
        return;
      }
      this.go(Math.min(this.step + 1, this.total - 1));
    },

    next() {
      if (!this.canContinue) return;
      this.advance();
    },

    back() {
      if (this.returnToReview) {
        this.returnToReview = false;
        this.go(this.total - 1);
        return;
      }
      if (this.step > 0) {
        this.go(this.step - 1);
        return;
      }
      this.closeFlow();
    },

    pickCategory(key, label, icon, tone) {
      this.category = key;
      this.categoryLabel = label;
      this.categoryIcon = icon;
      this.categoryTone = tone;
      setTimeout(() => {
        if (this.step === 0) this.advance();
      }, 140);
    },

    pickDate(value) {
      this.otherDate = false;
      this.date = value;
      setTimeout(() => {
        if (this.step === 1) this.advance();
      }, 140);
    },

    pickOther() {
      this.otherDate = true;
      this.showMonthOf(this.date || this.today);
    },

    pickCalendarDay(value) {
      if (!value || value > this.today) return;
      this.date = value;
    },

    toggleCurrency() {
      if (document.activeElement && document.activeElement.blur) {
        document.activeElement.blur();
      }
      this.currencyOpen = !this.currencyOpen;
    },

    pickCurrency(code, symbol, name) {
      this.currencyCode = code;
      this.currencySymbol = symbol;
      this.currencyName = name;
      this.currencyOther = false;
      this.otherCode = "";
      this.currencyOpen = false;
      this.$nextTick(() => {
        if (this.$refs.amount) this.$refs.amount.focus();
      });
    },

    pickOtherCurrency() {
      this.currencyOther = true;
      this.currencyOpen = false;
      this.cleanOtherCode();
      this.$nextTick(() => {
        if (this.$refs.otherCurrency) this.$refs.otherCurrency.focus();
      });
    },

    cleanOtherCode() {
      this.otherCode = this.otherCode
        .toUpperCase()
        .replace(/[^A-Z]/g, "")
        .slice(0, 3);
      this.currencyCode = this.otherCode;
      this.currencySymbol = this.otherCode;
      this.currencyName = "Other currency";
    },

    cleanAmount() {
      let value = this.amount.replace(/,/g, ".").replace(/[^\d.]/g, "");
      const dot = value.indexOf(".");
      if (dot !== -1) {
        value =
          value.slice(0, dot + 1) +
          value
            .slice(dot + 1)
            .replace(/\./g, "")
            .slice(0, 2);
      }
      this.amount = value.slice(0, 10);
    },

    onReceipt(event) {
      const file = event.target.files && event.target.files[0];
      if (!file) return;
      const isImage = file.type.startsWith("image/");
      if (!isImage && file.type !== "application/pdf") {
        this.clearReceipt();
        this.notify("Please choose an image or a PDF.");
        return;
      }
      if (file.size > 10 * 1024 * 1024) {
        this.clearReceipt();
        this.notify("That file is over 10 MB.");
        return;
      }
      this.revokeReceipt();
      this.receiptName = file.name;
      this.receiptSize = this.formatSize(file.size);
      this.receiptIsImage = isImage;
      this.receiptUrl = isImage ? URL.createObjectURL(file) : "";
    },

    revokeReceipt() {
      if (this.receiptUrl) URL.revokeObjectURL(this.receiptUrl);
      this.receiptUrl = "";
    },

    clearReceipt() {
      this.revokeReceipt();
      this.receiptName = "";
      this.receiptSize = "";
      this.receiptIsImage = false;
      if (this.$refs.receipt) this.$refs.receipt.value = "";
    },

    dragStart(event) {
      this.dragStartY = event.touches[0].clientY;
      this.dragOffset = 0;
    },

    dragMove(event) {
      const sheet = event.currentTarget.closest(".sheet");
      this.dragOffset = Math.max(0, event.touches[0].clientY - this.dragStartY);
      sheet.classList.add("is-dragging");
      sheet.style.transform = `translateY(${this.dragOffset}px)`;
    },

    dragEnd(event) {
      const sheet = event.currentTarget.closest(".sheet");
      const dismiss = this.dragOffset > 120;
      sheet.classList.remove("is-dragging");
      if (dismiss) {
        sheet.classList.add("is-dismissing");
        sheet.style.transform = "";
        this.close();
        setTimeout(() => sheet.classList.remove("is-dismissing"), 600);
      } else {
        sheet.style.transform = "";
      }
      this.dragOffset = 0;
    },
  }));
});
