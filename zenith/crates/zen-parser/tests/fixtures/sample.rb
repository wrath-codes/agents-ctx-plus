module Billing
  class Invoice < ApplicationRecord
    belongs_to :account
    has_many :line_items
    validates :number, presence: true
    scope :recent, -> { order(created_at: :desc) }

    def initialize(number)
      @number = number
    end

    def total
      line_items.sum(&:amount)
    end

    private

    def sync_ledger
      true
    end
  end

  class PaymentsController < ApplicationController
    before_action :authenticate_user!

    def create
      Invoice.create!
    end
  end
end
